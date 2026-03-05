use std::{borrow::Cow, collections::HashMap, collections::VecDeque, future::Future, task::Poll};

use super::{
    client_events::{ClientError, ClientRequest, ErrorKind, HostResponse},
    streaming::WsStreamHandle,
    Error, HostResult,
};
use futures::{pin_mut, FutureExt, Sink, SinkExt, Stream, StreamExt};
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
    queue: Vec<ClientRequest<'static>>,
    pending_streams: VecDeque<std::pin::Pin<Box<dyn Future<Output = HostResult> + Send>>>,
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
        // First, try to complete any pending stream assemblies.
        if let Some(fut) = self.pending_streams.front_mut() {
            if let Poll::Ready(result) = fut.as_mut().poll(cx) {
                self.pending_streams.pop_front();
                return Poll::Ready(Some(result));
            }
        }

        // Poll regular responses.
        match self.response_rx.poll_recv(cx) {
            Poll::Ready(Some(result)) => return Poll::Ready(Some(result)),
            Poll::Ready(None) => return Poll::Ready(None),
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
                self.pending_streams.push_back(fut);
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
        self.queue.push(item);
        Ok(())
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let mut queue = vec![];
        std::mem::swap(&mut queue, &mut self.queue);
        let mut error = false;
        while let Some(item) = queue.pop() {
            let f = self.request_tx.send(item);
            pin_mut!(f);
            match f.poll_unpin(cx) {
                Poll::Ready(Ok(_)) => continue,
                Poll::Ready(Err(_err)) => {
                    error = true;
                    break;
                }
                Poll::Pending => {}
            }
        }
        if error {
            self.queue.append(&mut queue);
            Poll::Ready(Err(ErrorKind::ChannelClosed.into()))
        } else {
            Poll::Ready(Ok(()))
        }
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
            queue: vec![],
            pending_streams: VecDeque::new(),
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
    pub async fn recv(&mut self) -> HostResult {
        tokio::select! {
            res = self.response_rx.recv() => {
                res.ok_or_else(|| ClientError::from(ErrorKind::ChannelClosed))?
            }
            handle = self.stream_rx.recv() => {
                let handle = handle.ok_or_else(|| ClientError::from(ErrorKind::ChannelClosed))?;
                let complete = handle
                    .assemble()
                    .await
                    .map_err(|e| ClientError::from(format!("{e}")))?;
                let inner: HostResult = bincode::deserialize(&complete)
                    .map_err(|e| ClientError::from(format!("{e}")))?;
                inner
            }
        }
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
    use super::streaming::{chunk_request, CHUNK_THRESHOLD};

    let req = req.ok_or(Error::ChannelClosed)?;
    let msg = bincode::serialize(&req)
        .map_err(Into::into)
        .map_err(Error::OtherError)?;

    if msg.len() > CHUNK_THRESHOLD {
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
