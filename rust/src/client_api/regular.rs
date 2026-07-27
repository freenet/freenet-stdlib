use std::{
    borrow::Cow, collections::HashMap, collections::VecDeque, future::Future, pin::Pin, task::Poll,
};

use super::{
    client_events::{ClientError, ClientRequest, ErrorKind, HostResponse},
    streaming::WsStreamHandle,
    Error, HostResult,
};
use futures::{stream::FuturesUnordered, Sink, SinkExt, Stream, StreamExt};
use tokio::{
    net::TcpStream,
    sync::mpsc::{self, Receiver, Sender},
};
use tokio_tungstenite::{
    tungstenite::{
        protocol::{frame::coding::CloseCode, CloseFrame},
        Message,
    },
    MaybeTlsStream, WebSocketStream,
};

type Connection = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct WebApi {
    request_tx: Sender<ClientRequest<'static>>,
    response_rx: Receiver<HostResult>,
    stream_rx: Receiver<WsStreamHandle>,
    queue: VecDeque<ClientRequest<'static>>,
    pending_streams: FuturesUnordered<Pin<Box<dyn Future<Output = HostResult> + Send>>>,
}

impl Drop for WebApi {
    fn drop(&mut self) {
        let req = self.request_tx.clone();
        tokio::spawn(async move {
            let _ = req.send(ClientRequest::Close).await;
        });
    }
}

impl Stream for WebApi {
    type Item = HostResult;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // Poll all pending stream assemblies concurrently.
        match self.pending_streams.poll_next_unpin(cx) {
            Poll::Ready(Some(result)) => return Poll::Ready(Some(result)),
            Poll::Ready(None) | Poll::Pending => {}
        }

        // Poll regular responses.
        match self.response_rx.poll_recv(cx) {
            Poll::Ready(Some(result)) => return Poll::Ready(Some(result)),
            // Closed and drained. Not terminal on its own: `request_handler`
            // drops both senders together, so a `WsStreamHandle` it queued
            // beforehand can still be waiting in `stream_rx`, and assemblies
            // may still be in flight. Fall through and let the stream arm below
            // decide, since it already ends the stream on the correct joint
            // condition (`stream_rx` closed AND no pending assemblies).
            // Returning `None` here discarded a complete streamed response.
            Poll::Ready(None) => {}
            Poll::Pending => {}
        }

        // Poll stream handles and spawn assembly as a pending future.
        match self.stream_rx.poll_recv(cx) {
            Poll::Ready(Some(handle)) => {
                let fut = Box::pin(async move {
                    let complete = handle
                        .assemble()
                        .await
                        .map_err(|e| ClientError::from(format!("{e}")))?;
                    let inner: HostResult = bincode::deserialize(&complete)
                        .map_err(|e| ClientError::from(format!("{e}")))?;
                    inner
                });
                self.pending_streams.push(fut);
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(None) if self.pending_streams.is_empty() => Poll::Ready(None),
            _ => Poll::Pending,
        }
    }
}

impl Sink<ClientRequest<'static>> for WebApi {
    type Error = ClientError;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        if self.queue.is_empty() {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }

    fn start_send(
        mut self: std::pin::Pin<&mut Self>,
        item: ClientRequest<'static>,
    ) -> Result<(), Self::Error> {
        self.queue.push_back(item);
        Ok(())
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        while let Some(item) = self.queue.pop_front() {
            match self.request_tx.try_send(item) {
                Ok(()) => continue,
                Err(mpsc::error::TrySendError::Full(item)) => {
                    self.queue.push_front(item);
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    return Poll::Ready(Err(ErrorKind::ChannelClosed.into()));
                }
            }
        }
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.poll_flush(cx)
    }
}

impl WebApi {
    pub fn start(connection: Connection) -> Self {
        let (request_tx, request_rx) = mpsc::channel(1);
        let (response_tx, response_rx) = mpsc::channel(1);
        let (stream_tx, stream_rx) = mpsc::channel(super::streaming::MAX_CONCURRENT_STREAMS);
        tokio::spawn(request_handler(
            request_rx,
            response_tx,
            stream_tx,
            connection,
        ));
        Self {
            request_tx,
            response_rx,
            stream_rx,
            queue: VecDeque::new(),
            pending_streams: FuturesUnordered::new(),
        }
    }

    /// Build a `WebApi` directly over channel halves, so a test can drive
    /// `recv()` against a chosen channel state without a socket or a live
    /// request handler.
    #[cfg(test)]
    fn from_parts(
        request_tx: Sender<ClientRequest<'static>>,
        response_rx: Receiver<HostResult>,
        stream_rx: Receiver<WsStreamHandle>,
    ) -> Self {
        Self {
            request_tx,
            response_rx,
            stream_rx,
            queue: VecDeque::new(),
            pending_streams: FuturesUnordered::new(),
        }
    }

    pub async fn send(&mut self, request: ClientRequest<'static>) -> Result<(), Error> {
        tracing::debug!(?request, "sending request");
        self.request_tx
            .send(request)
            .await
            .map_err(|_| ClientError::from(ErrorKind::ChannelClosed).into())
            .map_err(Error::OtherError)?;
        Ok(())
    }

    /// Receive the next host response.
    ///
    /// If the server sends a streamed response (StreamHeader + StreamChunks),
    /// this method transparently reassembles the full payload and returns the
    /// complete [`HostResponse`] — the caller does not need to handle streaming.
    ///
    /// For incremental consumption, use [`recv_stream()`](Self::recv_stream) instead.
    ///
    /// # Important
    ///
    /// `recv()` and [`recv_stream()`](Self::recv_stream) both consume from the
    /// internal stream channel. Calling both concurrently or alternating between
    /// them may cause responses to be delivered to the wrong consumer. Choose
    /// one consumption pattern per `WebApi` instance.
    ///
    /// A closed connection is reported as [`ErrorKind::ChannelClosed`] only once
    /// nothing is left to deliver. Anything the handler queued before shutting
    /// down is returned first, so the final error or response is not replaced by
    /// a generic channel error.
    ///
    /// # Cancel safety
    ///
    /// This method is **not** cancellation-safe. If the returned future is
    /// dropped while a streamed response is being reassembled, that response is
    /// lost. Do not use `recv()` directly as a `select!` branch that can be
    /// cancelled; drive it to completion, or use
    /// [`recv_stream()`](Self::recv_stream) and own the handle yourself.
    pub async fn recv(&mut self) -> HostResult {
        // Neither channel closing is terminal on its own. `request_handler`
        // delivers its final `HostResult` on `response_tx` and only *then*
        // returns, which drops both senders, so from the caller's side the
        // buffered response and the closure of the stream channel become ready
        // at the same instant. Treating whichever arm won as authoritative
        // discarded the other one's pending value: roughly half the time the
        // caller got a generic "comm channel between client/host closed"
        // instead of the real error the handler had just queued. Report a
        // closed connection only once BOTH channels are exhausted.
        //
        // `biased` makes the choice deterministic instead of leaving it to
        // `select!`'s random pick, and matches the precedence `Stream::poll_next`
        // above uses, so the two consumption paths agree. It also settles the
        // remaining variant of the same bug: at teardown a queued handle whose
        // chunk sender already died assembles to `StreamError::Truncated`, which
        // would otherwise bury the real error half the time.
        //
        // Draining responses first cannot starve the stream arm: `response_tx`
        // has capacity 1 and `handle_response_payload` awaits a permit, so the
        // handler blocks until the client consumes each response and cannot
        // produce an unbroken run of them while a handle waits.
        tokio::select! {
            biased;
            res = self.response_rx.recv() => {
                match res {
                    Some(res) => res,
                    None => {
                        let handle = self
                            .stream_rx
                            .recv()
                            .await
                            .ok_or_else(|| ClientError::from(ErrorKind::ChannelClosed))?;
                        Self::assemble_stream(handle).await
                    }
                }
            }
            handle = self.stream_rx.recv() => {
                match handle {
                    Some(handle) => Self::assemble_stream(handle).await,
                    None => self
                        .response_rx
                        .recv()
                        .await
                        .ok_or_else(|| ClientError::from(ErrorKind::ChannelClosed))?,
                }
            }
        }
    }

    /// Reassemble a streamed response into the complete [`HostResult`] the
    /// server sent.
    async fn assemble_stream(handle: WsStreamHandle) -> HostResult {
        let complete = handle
            .assemble()
            .await
            .map_err(|e| ClientError::from(format!("{e}")))?;
        let inner: HostResult =
            bincode::deserialize(&complete).map_err(|e| ClientError::from(format!("{e}")))?;
        inner
    }

    /// Receive the next streamed response as a [`WsStreamHandle`].
    ///
    /// Returns a handle for incremental consumption of a streamed response.
    /// Use [`WsStreamHandle::into_stream()`] for chunk-by-chunk processing or
    /// [`WsStreamHandle::assemble()`] to wait for the complete payload.
    ///
    /// Only returns when the server sends a `StreamHeader`; non-streamed
    /// responses are delivered through [`recv()`](Self::recv).
    ///
    /// # Important
    ///
    /// `recv_stream()` and [`recv()`](Self::recv) both consume from the internal
    /// stream channel. See [`recv()`](Self::recv) for details.
    pub async fn recv_stream(&mut self) -> Result<WsStreamHandle, Error> {
        self.stream_rx.recv().await.ok_or(Error::ChannelClosed)
    }

    #[doc(hidden)]
    pub async fn disconnect(self, cause: impl Into<Cow<'static, str>>) {
        let _ = self
            .request_tx
            .send(ClientRequest::Disconnect {
                cause: Some(cause.into()),
            })
            .await;
    }
}

async fn request_handler(
    mut request_rx: Receiver<ClientRequest<'static>>,
    mut response_tx: Sender<HostResult>,
    stream_tx: Sender<WsStreamHandle>,
    mut conn: Connection,
) {
    let mut reassembly = super::streaming::ReassemblyBuffer::new();
    let mut stream_senders: HashMap<u32, super::streaming::WsStreamSender> = HashMap::new();
    let mut next_stream_id: u32 = 0;

    let error = loop {
        tokio::select! {
            req = request_rx.recv() => {
                match process_request(&mut conn, req, &mut next_stream_id).await {
                    Ok(_) => continue,
                    Err(err) => break err,
                }
            }
            res = conn.next() => {
                match process_response(
                    &mut conn,
                    &mut response_tx,
                    &stream_tx,
                    &mut stream_senders,
                    res,
                    &mut reassembly,
                ).await {
                    Ok(_) => continue,
                    Err(err) => break err,
                }
            }
        }
    };
    tracing::debug!(?error, "request handler error");
    let error = match error {
        Error::ChannelClosed => ErrorKind::ChannelClosed.into(),
        Error::ConnectionClosed => ErrorKind::Disconnect.into(),
        other => ClientError::from(format!("{other}")),
    };
    let _ = response_tx.send(Err(error)).await;
}

async fn process_request(
    conn: &mut Connection,
    req: Option<ClientRequest<'static>>,
    next_stream_id: &mut u32,
) -> Result<(), Error> {
    use super::streaming::{chunk_request, ensure_chunkable, CHUNK_THRESHOLD};

    let req = req.ok_or(Error::ChannelClosed)?;
    let msg = bincode::serialize(&req)
        .map_err(Into::into)
        .map_err(Error::OtherError)?;

    if msg.len() > CHUNK_THRESHOLD {
        // Fail fast if the payload would exceed the node's reassembly cap
        // (ReassemblyBuffer::receive_chunk rejects total > MAX_TOTAL_CHUNKS on the
        // first chunk). Refuse to send anything rather than streaming the whole
        // oversized payload just to have the node reject it.
        //
        // Returning `Err` here breaks the request_handler loop (`break err`),
        // which tears down the WebApi connection. That is intentional and
        // acceptable: an over-cap request is unsendable and out-of-spec (>64 MiB,
        // already above the 50 MiB MAX_STATE_SIZE), and the error is still
        // delivered to the caller via `recv()` before teardown. (The browser/wasm
        // path surfaces the same error to the JS caller without tearing down the
        // connection.) We deliberately do not thread `response_tx` through here to
        // report the error per-request; the extra plumbing isn't worth it for a
        // request that cannot be sent regardless.
        ensure_chunkable(msg.len()).map_err(|e| Error::OtherError(e.into()))?;
        let stream_id = *next_stream_id;
        *next_stream_id = next_stream_id.wrapping_add(1);
        let chunks = chunk_request(msg, stream_id);
        for chunk in chunks {
            let chunk_bytes = bincode::serialize(&chunk)
                .map_err(Into::into)
                .map_err(Error::OtherError)?;
            conn.send(Message::Binary(chunk_bytes.into())).await?;
        }
    } else {
        conn.send(Message::Binary(msg.into())).await?;
    }

    if let ClientRequest::Disconnect { cause } = req {
        conn.close(cause.map(|c| CloseFrame {
            code: CloseCode::Normal,
            reason: format!("{c}").into(),
        }))
        .await?;
        return Err(Error::ConnectionClosed);
    } else if let ClientRequest::Close = req {
        conn.close(None).await?;
        return Err(Error::ConnectionClosed);
    }
    Ok(())
}

async fn process_response(
    conn: &mut Connection,
    response_tx: &mut Sender<HostResult>,
    stream_tx: &Sender<WsStreamHandle>,
    stream_senders: &mut HashMap<u32, super::streaming::WsStreamSender>,
    res: Option<Result<Message, tokio_tungstenite::tungstenite::Error>>,
    reassembly: &mut super::streaming::ReassemblyBuffer,
) -> Result<(), Error> {
    let res = res.ok_or(Error::ConnectionClosed)??;
    match res {
        Message::Binary(binary) => {
            handle_response_payload(&binary, response_tx, stream_tx, stream_senders, reassembly)
                .await
        }
        Message::Text(text) => {
            handle_response_payload(
                text.as_bytes(),
                response_tx,
                stream_tx,
                stream_senders,
                reassembly,
            )
            .await
        }
        Message::Ping(ping) => {
            conn.send(Message::Pong(ping)).await?;
            Ok(())
        }
        Message::Pong(_) => Ok(()),
        Message::Close(_) => Err(Error::ConnectionClosed),
        _ => Ok(()),
    }
}

async fn handle_response_payload(
    bytes: &[u8],
    response_tx: &mut Sender<HostResult>,
    stream_tx: &Sender<WsStreamHandle>,
    stream_senders: &mut HashMap<u32, super::streaming::WsStreamSender>,
    reassembly: &mut super::streaming::ReassemblyBuffer,
) -> Result<(), Error> {
    let response: HostResult = bincode::deserialize(bytes)?;
    match response {
        Ok(HostResponse::StreamHeader {
            stream_id,
            total_bytes,
            content,
        }) => {
            // Cap open streams to prevent unbounded growth from abandoned streams
            if stream_senders.len() >= super::streaming::MAX_CONCURRENT_STREAMS {
                tracing::warn!("too many open stream senders, evicting one");
                if let Some(&id) = stream_senders.keys().next() {
                    stream_senders.remove(&id);
                    reassembly.remove_stream(id);
                }
            }
            let (handle, sender) = super::streaming::ws_stream_pair(content, total_bytes);
            stream_senders.insert(stream_id, sender);
            match stream_tx.try_send(handle) {
                Ok(()) => Ok(()),
                Err(mpsc::error::TrySendError::Full(_)) => {
                    tracing::warn!(
                        stream_id,
                        "stream_tx full, falling back to transparent reassembly"
                    );
                    // Remove sender so subsequent chunks go through ReassemblyBuffer
                    stream_senders.remove(&stream_id);
                    Ok(())
                }
                Err(mpsc::error::TrySendError::Closed(_)) => Err(Error::ChannelClosed),
            }
        }
        Ok(HostResponse::StreamChunk {
            stream_id,
            index,
            total,
            data,
        }) => {
            // If we have a sender for this stream_id, it was preceded by a StreamHeader
            // → route chunks to the WsStreamSender for app-level streaming.
            if let Some(sender) = stream_senders.get(&stream_id) {
                if let Err(e) = sender.send_chunk(data) {
                    tracing::warn!(stream_id, "stream chunk send failed: {e}");
                    stream_senders.remove(&stream_id);
                    return Ok(());
                }
                // Drop sender on last chunk so the handle's rx closes
                if index + 1 == total {
                    stream_senders.remove(&stream_id);
                }
                Ok(())
            } else {
                // No StreamHeader seen → transparent reassembly (backward compat)
                match reassembly
                    .receive_chunk(stream_id, index, total, data)
                    .map_err(|e| Error::OtherError(e.into()))?
                {
                    Some(complete) => {
                        let inner: HostResult = bincode::deserialize(&complete)?;
                        response_tx
                            .send(inner)
                            .await
                            .map_err(|_| Error::ChannelClosed)?;
                        Ok(())
                    }
                    None => Ok(()),
                }
            }
        }
        other => {
            response_tx
                .send(other)
                .await
                .map_err(|_| Error::ChannelClosed)?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use crate::client_api::HostResponse;

    use super::*;
    use std::{net::Ipv4Addr, time::Duration};
    use tokio::net::TcpListener;

    /// Bind to an OS-assigned port and return the listener + port.
    async fn bind_free_port() -> (TcpListener, u16) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0u16))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        (listener, port)
    }

    struct Server {
        recv: bool,
        listener: TcpListener,
    }

    impl Server {
        async fn new(listener: TcpListener, recv: bool) -> Self {
            Server { recv, listener }
        }

        async fn listen(
            self,
            tx: tokio::sync::oneshot::Sender<()>,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let (stream, _) =
                tokio::time::timeout(Duration::from_secs(5), self.listener.accept()).await??;
            let mut stream = tokio_tungstenite::accept_async(stream).await?;

            if !self.recv {
                let res: HostResult = Ok(HostResponse::Ok);
                let bytes = bincode::serialize(&res)?;
                stream.send(Message::Binary(bytes.into())).await?;
            }

            let Message::Binary(msg) = stream.next().await.ok_or_else(|| "no msg".to_owned())??
            else {
                return Err("wrong msg".to_owned().into());
            };

            let _req: ClientRequest = bincode::deserialize(&msg)?;
            tx.send(()).map_err(|_| "couldn't error".to_owned())?;
            Ok(())
        }
    }

    /// Build a serialized GetResponse payload of the given size and fill byte.
    fn build_test_payload(
        payload_size: usize,
        fill: u8,
    ) -> (Vec<u8>, crate::contract_interface::ContractKey) {
        use crate::contract_interface::{ContractCode, ContractKey, WrappedState};
        use crate::parameters::Parameters;

        let state = WrappedState::new(vec![fill; payload_size]);
        let code = ContractCode::from(vec![1, 2, 3]);
        let key = ContractKey::from_params_and_code(Parameters::from(vec![]), &code);
        let res: HostResult = Ok(HostResponse::ContractResponse(
            crate::client_api::ContractResponse::GetResponse {
                key,
                contract: None,
                state,
            },
        ));
        (bincode::serialize(&res).unwrap(), key)
    }

    /// Accept a WS connection and send chunks (optionally preceded by a StreamHeader).
    async fn serve_chunked_response(
        listener: TcpListener,
        payload_size: usize,
        fill: u8,
        send_header: bool,
        tx: tokio::sync::oneshot::Sender<()>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::client_api::streaming;

        let (tcp_stream, _) =
            tokio::time::timeout(Duration::from_secs(5), listener.accept()).await??;
        let mut stream = tokio_tungstenite::accept_async(tcp_stream).await?;

        let (serialized, key) = build_test_payload(payload_size, fill);
        let stream_id = 0u32;

        if send_header {
            use crate::client_api::client_events::StreamContent;
            let header: HostResult = Ok(HostResponse::StreamHeader {
                stream_id,
                total_bytes: serialized.len() as u64,
                content: StreamContent::GetResponse {
                    key,
                    includes_contract: false,
                },
            });
            let header_bytes = bincode::serialize(&header)?;
            stream.send(Message::Binary(header_bytes.into())).await?;
        }

        let chunks = streaming::chunk_response(serialized, stream_id);
        assert!(chunks.len() > 1, "payload should produce multiple chunks");
        for chunk in chunks {
            let chunk_result: HostResult = Ok(chunk);
            let chunk_bytes = bincode::serialize(&chunk_result)?;
            stream.send(Message::Binary(chunk_bytes.into())).await?;
        }

        let msg = tokio::time::timeout(Duration::from_secs(5), stream.next()).await;
        drop(msg);
        tx.send(()).map_err(|_| "signal failed".to_owned())?;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_recv_chunked() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::client_api::ContractResponse;

        let payload_size = 600 * 1024;
        let (listener, port) = bind_free_port().await;
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let server_result = tokio::task::spawn(serve_chunked_response(
            listener,
            payload_size,
            0xAB,
            false,
            tx,
        ));
        let (ws_conn, _) =
            tokio_tungstenite::connect_async(format!("ws://localhost:{port}/")).await?;
        let mut client = WebApi::start(ws_conn);

        let response = client.recv().await?;
        match response {
            HostResponse::ContractResponse(ContractResponse::GetResponse { state, .. }) => {
                assert_eq!(state.size(), payload_size);
                assert!(state.as_ref().iter().all(|&b| b == 0xAB));
            }
            other => panic!("expected GetResponse, got {other:?}"),
        }

        client
            .send(ClientRequest::Disconnect { cause: None })
            .await?;
        tokio::time::timeout(Duration::from_secs(5), rx).await??;
        tokio::time::timeout(Duration::from_secs(5), server_result).await???;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_recv_stream_header() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::client_api::ContractResponse;

        let payload_size = 600 * 1024;
        let (listener, port) = bind_free_port().await;
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let server_result = tokio::task::spawn(serve_chunked_response(
            listener,
            payload_size,
            0xCD,
            true,
            tx,
        ));
        let (ws_conn, _) =
            tokio_tungstenite::connect_async(format!("ws://localhost:{port}/")).await?;
        let mut client = WebApi::start(ws_conn);

        // Use recv_stream() to get the handle
        let handle = client.recv_stream().await.unwrap();
        assert!(handle.total_bytes() >= payload_size as u64);

        // Assemble and verify
        let complete = handle.assemble().await.unwrap();
        let inner: HostResult = bincode::deserialize(&complete)?;
        match inner? {
            HostResponse::ContractResponse(ContractResponse::GetResponse { state, .. }) => {
                assert_eq!(state.size(), payload_size);
                assert!(state.as_ref().iter().all(|&b| b == 0xCD));
            }
            other => panic!("expected GetResponse, got {other:?}"),
        }

        client
            .send(ClientRequest::Disconnect { cause: None })
            .await?;
        tokio::time::timeout(Duration::from_secs(5), rx).await??;
        tokio::time::timeout(Duration::from_secs(5), server_result).await???;
        Ok(())
    }

    /// Tests that recv() transparently assembles StreamHeader+StreamChunk flows.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_recv_transparent_stream_header(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::client_api::ContractResponse;

        let payload_size = 600 * 1024;
        let (listener, port) = bind_free_port().await;
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let server_result = tokio::task::spawn(serve_chunked_response(
            listener,
            payload_size,
            0xCD,
            true,
            tx,
        ));
        let (ws_conn, _) =
            tokio_tungstenite::connect_async(format!("ws://localhost:{port}/")).await?;
        let mut client = WebApi::start(ws_conn);

        // Use recv() which should auto-assemble the stream
        let response = client.recv().await?;
        match response {
            HostResponse::ContractResponse(ContractResponse::GetResponse { state, .. }) => {
                assert_eq!(state.size(), payload_size);
                assert!(state.as_ref().iter().all(|&b| b == 0xCD));
            }
            other => panic!("expected GetResponse, got {other:?}"),
        }

        client
            .send(ClientRequest::Disconnect { cause: None })
            .await?;
        tokio::time::timeout(Duration::from_secs(5), rx).await??;
        tokio::time::timeout(Duration::from_secs(5), server_result).await???;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_send() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (listener, port) = bind_free_port().await;
        let server = Server::new(listener, true).await;
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let server_result = tokio::task::spawn(server.listen(tx));
        let (ws_conn, _) =
            tokio_tungstenite::connect_async(format!("ws://localhost:{port}/")).await?;
        let mut client = WebApi::start(ws_conn);

        client
            .send(ClientRequest::Disconnect { cause: None })
            .await?;
        tokio::time::timeout(Duration::from_secs(5), rx).await??;
        tokio::time::timeout(Duration::from_secs(5), server_result).await???;
        Ok(())
    }

    /// Regression test pinning the send-path chunk-limit guard in
    /// `process_request`. An oversized request (serialized length above the
    /// 64 MiB `MAX_TOTAL_CHUNKS * CHUNK_SIZE` cap) must be rejected locally with a
    /// `TotalChunksTooLarge` error *before* any chunk is streamed.
    ///
    /// Acceptance criterion: if the `ensure_chunkable(...)` call is removed from
    /// `process_request`, the client instead streams the whole payload and the
    /// delivered error no longer mentions "exceeds maximum", so this test fails.
    /// (Verified by temporarily removing the guard.)
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_send_oversized_rejected_before_streaming(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::client_api::streaming::{CHUNK_SIZE, MAX_TOTAL_CHUNKS};
        use crate::client_api::ContractRequest;
        use crate::prelude::{
            ContractCode, ContractContainer, ContractWasmAPIVersion, Parameters, RelatedContracts,
            WrappedContract, WrappedState,
        };
        use std::sync::Arc;

        let (listener, port) = bind_free_port().await;
        // The server only completes the WS handshake, then drains anything the
        // client sends until the client disconnects or a short idle window
        // elapses, then drops. With the guard present the client sends nothing (it
        // fails locally); if the guard were removed it would stream ~64 MiB of
        // chunks, which this drain loop absorbs so the send path can't deadlock.
        let server = tokio::task::spawn(async move {
            let (tcp, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
                .await
                .expect("accept timed out")
                .expect("accept failed");
            let mut ws = tokio_tungstenite::accept_async(tcp)
                .await
                .expect("ws handshake failed");
            let _ = tokio::time::timeout(Duration::from_secs(3), async {
                while let Some(Ok(_)) = ws.next().await {}
            })
            .await;
        });

        let (ws_conn, _) =
            tokio_tungstenite::connect_async(format!("ws://localhost:{port}/")).await?;
        let mut client = WebApi::start(ws_conn);

        // A Put whose state is one byte over the 64 MiB (256 * 256 KiB) chunk cap;
        // serialization overhead only makes it larger, so it needs more than
        // MAX_TOTAL_CHUNKS chunks. This is the single deliberate ~64 MiB alloc.
        let oversized_state = MAX_TOTAL_CHUNKS as usize * CHUNK_SIZE + 1;
        let code = Arc::new(ContractCode::from(vec![1, 2, 3]));
        let contract = ContractContainer::Wasm(ContractWasmAPIVersion::V1(WrappedContract::new(
            code,
            Parameters::from(Vec::new()),
        )));
        let request = ClientRequest::ContractOp(ContractRequest::Put {
            contract,
            state: WrappedState::new(vec![0u8; oversized_state]),
            related_contracts: RelatedContracts::new(),
            subscribe: false,
            blocking_subscribe: false,
        });

        client.send(request).await?;
        let err = client
            .recv()
            .await
            .expect_err("oversized request must be rejected, not streamed");
        let msg = format!("{err}");
        assert!(
            msg.contains("exceeds maximum"),
            "expected a TotalChunksTooLarge error from the send-path guard, got: {msg}"
        );

        drop(client);
        let _ = tokio::time::timeout(Duration::from_secs(5), server).await;
        Ok(())
    }

    /// `recv()` must not throw away a delivered response because the *other*
    /// internal channel closed first.
    ///
    /// `request_handler` sends its final `HostResult` on `response_tx` and only
    /// then returns, dropping both senders. Whenever the caller is scheduled
    /// after the handler has finished (the normal case on a loaded machine)
    /// both arms of `recv()`'s `select!` are ready at once. The old code treated
    /// a closed `stream_rx` as terminal, so `select!`'s random pick discarded
    /// the real error sitting in `response_rx` about half the time. That is what
    /// made `test_send_oversized_rejected_before_streaming` fail 6 runs in 25 of
    /// the full suite under load, reporting "comm channel between client/host
    /// closed" in place of the guard's own message.
    ///
    /// This reproduces that interleaving directly rather than racing for it:
    /// the response is buffered and both senders are dropped *before* `recv()`
    /// is called, so the failing state is set up deterministically. The loop
    /// keeps the pin sharp against a partial revert that restores the random
    /// pick: one iteration would catch that only half the time, 200 make a
    /// false pass a 2^-200 event.
    #[tokio::test]
    async fn recv_delivers_buffered_response_when_stream_channel_closes_first() {
        for i in 0..200 {
            let (request_tx, _request_rx) = mpsc::channel::<ClientRequest<'static>>(1);
            let (response_tx, response_rx) = mpsc::channel::<HostResult>(1);
            let (stream_tx, stream_rx) = mpsc::channel::<WsStreamHandle>(1);

            // Exactly the shutdown ordering `request_handler` performs: deliver
            // the final result, then drop the senders on task exit.
            response_tx
                .send(Err(ClientError::from(
                    "request exceeds maximum".to_string(),
                )))
                .await
                .expect("buffering the response must succeed");
            drop(response_tx);
            drop(stream_tx);

            let mut client = WebApi::from_parts(request_tx, response_rx, stream_rx);
            let err = client
                .recv()
                .await
                .expect_err("the buffered error must be delivered");
            assert!(
                format!("{err}").contains("request exceeds maximum"),
                "iteration {i}: recv() discarded the buffered response and reported a closed \
                 channel instead, got: {err}"
            );
        }
    }

    /// The mirror case: a stream handle queued before shutdown must survive the
    /// response channel closing and draining first.
    ///
    /// This is the other half of the fix, and the more valuable half: what is
    /// recovered here is a complete streamed *success* response, not just an
    /// error string. It is also the only test that reaches the response arm's
    /// `None` branch, since the test above always leaves a response buffered.
    /// Without it, replacing that branch with the old terminal `ChannelClosed`
    /// leaves the whole suite green.
    ///
    /// Reachable in production: `handle_response_payload` queues a handle on
    /// `stream_tx` when a `StreamHeader` arrives, and the connection can then
    /// close before the caller has consumed it.
    #[tokio::test]
    async fn recv_delivers_queued_stream_after_response_channel_closes() {
        use crate::client_api::client_events::StreamContent;
        use crate::client_api::streaming::ws_stream_pair;

        let payload: HostResult = Err(ClientError::from("streamed payload".to_string()));
        let encoded = bincode::serialize(&payload).expect("serialize the streamed HostResult");

        let (handle, sender) = ws_stream_pair(StreamContent::Raw, encoded.len() as u64);
        sender
            .send_chunk(encoded.into())
            .expect("feeding the stream must succeed");
        drop(sender);

        let (request_tx, _request_rx) = mpsc::channel::<ClientRequest<'static>>(1);
        let (response_tx, response_rx) = mpsc::channel::<HostResult>(1);
        let (stream_tx, stream_rx) = mpsc::channel::<WsStreamHandle>(1);
        stream_tx
            .send(handle)
            .await
            .expect("queueing the handle must succeed");
        // The response channel closes empty while the handle is still queued.
        drop(response_tx);
        drop(stream_tx);

        let mut client = WebApi::from_parts(request_tx, response_rx, stream_rx);
        let err = client
            .recv()
            .await
            .expect_err("the queued stream must be assembled and delivered");
        assert!(
            format!("{err}").contains("streamed payload"),
            "recv() discarded the queued stream handle and reported a closed channel \
             instead, got: {err}"
        );
    }

    /// The same precedence, through the other public consumption path.
    ///
    /// `WebApi` is also a [`Stream`], and `poll_next` had its own copy of the
    /// bug: it returned `Poll::Ready(None)` the moment `response_rx` closed,
    /// without checking whether a handle was still queued in `stream_rx` or an
    /// assembly was still in flight. There it was deterministic rather than a
    /// 50/50 race, so it discarded a complete streamed response every time.
    #[tokio::test]
    async fn poll_next_delivers_queued_stream_after_response_channel_closes() {
        use crate::client_api::client_events::StreamContent;
        use crate::client_api::streaming::ws_stream_pair;

        let payload: HostResult = Err(ClientError::from("streamed payload".to_string()));
        let encoded = bincode::serialize(&payload).expect("serialize the streamed HostResult");

        let (handle, sender) = ws_stream_pair(StreamContent::Raw, encoded.len() as u64);
        sender
            .send_chunk(encoded.into())
            .expect("feeding the stream must succeed");
        drop(sender);

        let (request_tx, _request_rx) = mpsc::channel::<ClientRequest<'static>>(1);
        let (response_tx, response_rx) = mpsc::channel::<HostResult>(1);
        let (stream_tx, stream_rx) = mpsc::channel::<WsStreamHandle>(1);
        stream_tx
            .send(handle)
            .await
            .expect("queueing the handle must succeed");
        drop(response_tx);
        drop(stream_tx);

        let mut client = WebApi::from_parts(request_tx, response_rx, stream_rx);
        let item = client
            .next()
            .await
            .expect("the stream must yield the queued response, not end");
        let err = item.expect_err("the streamed payload is an Err in this fixture");
        assert!(
            format!("{err}").contains("streamed payload"),
            "poll_next ended the stream and discarded the queued handle, got: {err}"
        );
    }

    /// A closed stream channel must not end a still-live connection.
    ///
    /// This pins the stream arm's `None` branch. With `biased;` that arm is
    /// only reached when `response_rx` is open but empty, so "the stream side
    /// is finished" has to mean "keep waiting for responses", not "report the
    /// connection closed": the response channel is still live and about to
    /// deliver. Reporting `ChannelClosed` here would drop a healthy connection.
    #[tokio::test]
    async fn recv_keeps_waiting_for_responses_after_the_stream_channel_closes() {
        let (request_tx, _request_rx) = mpsc::channel::<ClientRequest<'static>>(1);
        let (response_tx, response_rx) = mpsc::channel::<HostResult>(1);
        let (stream_tx, stream_rx) = mpsc::channel::<WsStreamHandle>(1);
        // The stream side is done; the response side is still open and empty.
        drop(stream_tx);

        let mut client = WebApi::from_parts(request_tx, response_rx, stream_rx);
        let (received, _) = tokio::join!(client.recv(), async {
            response_tx
                .send(Err(ClientError::from("late response".to_string())))
                .await
                .expect("sending the late response must succeed");
        });

        let err = received.expect_err("the late response is an Err in this fixture");
        assert!(
            format!("{err}").contains("late response"),
            "recv() gave up on a live response channel because the stream channel \
             closed, got: {err}"
        );
    }

    /// `ChannelClosed` is still what a caller gets once genuinely nothing is
    /// left to deliver, so the tests above cannot be satisfied by never
    /// reporting a closed connection at all.
    #[tokio::test]
    async fn recv_reports_closed_only_when_both_channels_are_exhausted() {
        let (request_tx, _request_rx) = mpsc::channel::<ClientRequest<'static>>(1);
        let (_response_tx, response_rx) = mpsc::channel::<HostResult>(1);
        let (_stream_tx, stream_rx) = mpsc::channel::<WsStreamHandle>(1);
        drop(_response_tx);
        drop(_stream_tx);

        let mut client = WebApi::from_parts(request_tx, response_rx, stream_rx);
        let err = client
            .recv()
            .await
            .expect_err("both channels closed and empty must surface as ChannelClosed");
        assert!(
            format!("{err}").contains("comm channel between client/host closed"),
            "expected the channel-closed error once nothing is left to deliver, got: {err}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_recv() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (listener, port) = bind_free_port().await;
        let server = Server::new(listener, false).await;
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let server_result = tokio::task::spawn(server.listen(tx));
        let (ws_conn, _) =
            tokio_tungstenite::connect_async(format!("ws://localhost:{port}/")).await?;
        let mut client = WebApi::start(ws_conn);

        let _res = client.recv().await;
        client
            .send(ClientRequest::Disconnect { cause: None })
            .await?;
        tokio::time::timeout(Duration::from_secs(5), rx).await??;
        tokio::time::timeout(Duration::from_secs(5), server_result).await???;
        Ok(())
    }
}
