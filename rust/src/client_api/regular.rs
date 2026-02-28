use std::{borrow::Cow, task::Poll};

use super::{
    client_events::{ClientError, ClientRequest, ErrorKind},
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
    queue: Vec<ClientRequest<'static>>,
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
        self.response_rx.poll_recv(cx)
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
        tokio::spawn(request_handler(request_rx, response_tx, connection));
        Self {
            request_tx,
            response_rx,
            queue: vec![],
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

    pub async fn recv(&mut self) -> HostResult {
        let res = self.response_rx.recv().await;
        res.ok_or_else(|| ClientError::from(ErrorKind::ChannelClosed))?
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
    mut conn: Connection,
) {
    let mut reassembly = super::ws_streaming::ChunkReassemblyBuffer::new();

    let error = loop {
        tokio::select! {
            req = request_rx.recv() => {
                match process_request(&mut conn, req).await {
                    Ok(_) => continue,
                    Err(err) => break err,
                }
            }
            res = conn.next() => {
                match process_response(&mut conn, &mut response_tx, res, &mut reassembly).await {
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

#[inline]
async fn process_request(
    conn: &mut Connection,
    req: Option<ClientRequest<'static>>,
) -> Result<(), Error> {
    use super::ws_streaming::{self, CHUNK_THRESHOLD};

    let req = req.ok_or(Error::ChannelClosed)?;
    let msg = bincode::serialize(&req)
        .map_err(Into::into)
        .map_err(Error::OtherError)?;

    if msg.len() > CHUNK_THRESHOLD {
        let chunks = ws_streaming::chunk_payload(&msg);
        for chunk in chunks {
            conn.send(Message::Binary(chunk.into())).await?;
        }
    } else {
        let wrapped = ws_streaming::wrap_complete(msg);
        conn.send(Message::Binary(wrapped.into())).await?;
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

#[inline]
async fn process_response(
    conn: &mut Connection,
    response_tx: &mut Sender<HostResult>,
    res: Option<Result<Message, tokio_tungstenite::tungstenite::Error>>,
    reassembly: &mut super::ws_streaming::ChunkReassemblyBuffer,
) -> Result<(), Error> {
    use super::ws_streaming::{self, StreamMessage};

    let res = res.ok_or(Error::ConnectionClosed)??;
    match res {
        Message::Text(msg) => {
            let bytes = match ws_streaming::parse_message(msg.as_bytes())
                .map_err(|e| Error::OtherError(e.into()))?
            {
                StreamMessage::Complete(payload) => payload.to_vec(),
                StreamMessage::Chunk {
                    total_chunks,
                    payload,
                } => match reassembly
                    .receive_chunk(total_chunks, payload)
                    .map_err(|e| Error::OtherError(e.into()))?
                {
                    Some(complete) => complete,
                    None => return Ok(()),
                },
            };
            let response: HostResult = bincode::deserialize(&bytes)?;
            response_tx
                .send(response)
                .await
                .map_err(|_| Error::ChannelClosed)?;
        }
        Message::Binary(binary) => {
            let bytes = match ws_streaming::parse_message(&binary)
                .map_err(|e| Error::OtherError(e.into()))?
            {
                StreamMessage::Complete(payload) => payload.to_vec(),
                StreamMessage::Chunk {
                    total_chunks,
                    payload,
                } => match reassembly
                    .receive_chunk(total_chunks, payload)
                    .map_err(|e| Error::OtherError(e.into()))?
                {
                    Some(complete) => complete,
                    None => return Ok(()),
                },
            };
            let response: HostResult = bincode::deserialize(&bytes)?;
            response_tx
                .send(response)
                .await
                .map_err(|_| Error::ChannelClosed)?;
        }
        Message::Ping(ping) => {
            conn.send(Message::Pong(ping)).await?;
        }
        Message::Pong(_) => {}
        Message::Close(_) => return Err(Error::ConnectionClosed),
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::client_api::HostResponse;

    use super::*;
    use std::{net::Ipv4Addr, sync::atomic::AtomicU16, time::Duration};
    use tokio::net::TcpListener;

    static PORT: AtomicU16 = AtomicU16::new(65495);

    struct Server {
        recv: bool,
        listener: TcpListener,
    }

    impl Server {
        async fn new(port: u16, recv: bool) -> Self {
            let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port))
                .await
                .unwrap();
            Server { recv, listener }
        }

        async fn listen(
            self,
            tx: tokio::sync::oneshot::Sender<()>,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            use crate::client_api::ws_streaming;

            let (stream, _) =
                tokio::time::timeout(Duration::from_millis(10), self.listener.accept()).await??;
            let mut stream = tokio_tungstenite::accept_async(stream).await?;

            if !self.recv {
                let res: HostResult = Ok(HostResponse::Ok);
                let req = bincode::serialize(&res)?;
                let wrapped = ws_streaming::wrap_complete(req);
                stream.send(Message::Binary(wrapped.into())).await?;
            }

            let Message::Binary(msg) = stream.next().await.ok_or_else(|| "no msg".to_owned())??
            else {
                return Err("wrong msg".to_owned().into());
            };

            // Unwrap the streaming envelope
            let payload = match ws_streaming::parse_message(&msg)? {
                ws_streaming::StreamMessage::Complete(data) => data.to_vec(),
                ws_streaming::StreamMessage::Chunk { .. } => {
                    return Err("unexpected chunk in test".to_owned().into());
                }
            };

            let _req: ClientRequest = bincode::deserialize(&payload)?;
            tx.send(()).map_err(|_| "couldn't error".to_owned())?;
            Ok(())
        }
    }

    struct ChunkedServer {
        listener: TcpListener,
        payload_size: usize,
    }

    impl ChunkedServer {
        async fn new(port: u16, payload_size: usize) -> Self {
            let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port))
                .await
                .unwrap();
            ChunkedServer {
                listener,
                payload_size,
            }
        }

        async fn listen(
            self,
            tx: tokio::sync::oneshot::Sender<()>,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            use crate::client_api::ws_streaming;
            use crate::contract_interface::{ContractCode, ContractKey, WrappedState};
            use crate::parameters::Parameters;

            let (stream, _) =
                tokio::time::timeout(Duration::from_millis(100), self.listener.accept()).await??;
            let mut stream = tokio_tungstenite::accept_async(stream).await?;

            let state = WrappedState::new(vec![0xAB; self.payload_size]);
            let code = ContractCode::from(vec![1, 2, 3]);
            let key = ContractKey::from_params_and_code(Parameters::from(vec![]), &code);
            let res: HostResult = Ok(HostResponse::ContractResponse(
                crate::client_api::ContractResponse::GetResponse {
                    key,
                    contract: None,
                    state,
                },
            ));
            let serialized = bincode::serialize(&res)?;

            // Send as chunks
            let chunks = ws_streaming::chunk_payload(&serialized);
            assert!(chunks.len() > 1, "payload should produce multiple chunks");
            for chunk in chunks {
                stream.send(Message::Binary(chunk.into())).await?;
            }

            // Wait for client disconnect
            let msg = tokio::time::timeout(Duration::from_millis(100), stream.next()).await;
            drop(msg);
            tx.send(()).map_err(|_| "signal failed".to_owned())?;
            Ok(())
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_recv_chunked() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::client_api::ContractResponse;

        let port = PORT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let payload_size = 600 * 1024; // 600 KiB state → multiple chunks
        let server = ChunkedServer::new(port, payload_size).await;
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let server_result = tokio::task::spawn(server.listen(tx));
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
        tokio::time::timeout(Duration::from_millis(100), rx).await??;
        tokio::time::timeout(Duration::from_millis(100), server_result).await???;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_send() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let port = PORT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let server = Server::new(port, true).await;
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let server_result = tokio::task::spawn(server.listen(tx));
        let (ws_conn, _) =
            tokio_tungstenite::connect_async(format!("ws://localhost:{port}/")).await?;
        let mut client = WebApi::start(ws_conn);

        client
            .send(ClientRequest::Disconnect { cause: None })
            .await?;
        tokio::time::timeout(Duration::from_millis(10), rx).await??;
        tokio::time::timeout(Duration::from_millis(10), server_result).await???;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_recv() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let port = PORT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let server = Server::new(port, false).await;
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let server_result = tokio::task::spawn(server.listen(tx));
        let (ws_conn, _) =
            tokio_tungstenite::connect_async(format!("ws://localhost:{port}/")).await?;
        let mut client = WebApi::start(ws_conn);

        let _res = client.recv().await;
        client
            .send(ClientRequest::Disconnect { cause: None })
            .await?;
        tokio::time::timeout(Duration::from_millis(10), rx).await??;
        tokio::time::timeout(Duration::from_millis(10), server_result).await???;
        Ok(())
    }
}
