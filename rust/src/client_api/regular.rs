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
    let error = loop {
        tokio::select! {
            req = request_rx.recv() => {
                match process_request(&mut conn, req).await {
                    Ok(_) => continue,
                    Err(err) => break err,
                }
            }
            res = conn.next() => {
                match process_response(&mut conn, &mut response_tx, res).await {
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
    let req = req.ok_or(Error::ChannelClosed)?;
    let msg = bincode::serialize(&req)
        .map_err(Into::into)
        .map_err(Error::OtherError)?;
    conn.send(Message::Binary(msg.into())).await?;
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
) -> Result<(), Error> {
    let res = res.ok_or(Error::ConnectionClosed)??;
    match res {
        Message::Text(msg) => {
            let response: HostResult = bincode::deserialize(msg.as_bytes())?;
            response_tx
                .send(response)
                .await
                .map_err(|_| Error::ChannelClosed)?;
        }
        Message::Binary(binary) => {
            let response: HostResult = bincode::deserialize(&binary)?;
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
            let (stream, _) =
                tokio::time::timeout(Duration::from_millis(10), self.listener.accept()).await??;
            let mut stream = tokio_tungstenite::accept_async(stream).await?;

            if !self.recv {
                let res: HostResult = Ok(HostResponse::Ok);
                let req = bincode::serialize(&res)?;
                stream.send(Message::Binary(req.into())).await?;
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
