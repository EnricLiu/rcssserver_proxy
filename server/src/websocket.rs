use std::net::SocketAddr;
use std::sync::Arc;

use dashmap::DashMap;

use tokio::select;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::{RwLock, mpsc};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use futures::stream::SplitStream;
use futures::{SinkExt, StreamExt};

use tungstenite::Message;
use tokio_tungstenite::{WebSocketStream, accept_async};

use crate::config::Config;
use crate::error::{Error, Result};

use common::signal::Signal;
use log::{debug, info, trace, warn};

pub struct Server {
    pub config: Config,
    pub listening: RwLock<bool>,
    pub clients: Arc<DashMap<SocketAddr,
        (JoinHandle<Result<()>>, JoinHandle<Result<()>>, JoinHandle<Result<()>>)
    >>,
}

impl Server {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            listening: RwLock::new(false),
            clients: Arc::new(DashMap::new()),
        }
    }

    pub async fn listen(&self) -> Result<JoinHandle<()>> {
        if *self.listening.read().await {
            trace!("Server already started");
            return Err(Error::AlreadyStarted(self.config.clone()));
        }

        // _listening lockup
        let _listening = self.listening.write().await;
        let clients = Arc::clone(&self.clients);
        let ws_listener = TcpListener::bind(self.config.ws_url).await?;
        debug!("Starting WebSocket Server on {}", self.config.ws_url);

        let config = self.config.clone();
        let handle = tokio::spawn(async move {
            while let Ok((stream, addr)) = ws_listener.accept().await {
                info!("New WebSocket connection from {}", addr);
                Self::handle_accept(stream, addr, config.clone(), clients.clone()).await;
            }
        });

        Ok(handle) // _listening free
    }

    async fn handle_accept(
        tcp: TcpStream,
        addr: SocketAddr,
        config: Config,
        clients: Arc<DashMap<SocketAddr,
            (JoinHandle<Result<()>>,JoinHandle<Result<()>>,JoinHandle<Result<()>>)>>,
    ) -> () {
        let cancel = CancellationToken::new();

        let (ws_tx, mut ws_rx, ws_mpsc_task) = {
            let websocket = accept_async(tcp).await.unwrap();
            debug!("[{addr}]: ws conn established");
            split_ws_spawn_mpsc::<Signal, _>(websocket, 256, Some(addr)).await
        };

        let Config {
            udp_proxy_url,
            udp_rcsss_url,
            ..
        } = config;

        let (udp_tx, udp_rx) = {
            let udp_client = UdpSocket::bind(udp_proxy_url)
                .await
                .expect("Failed to bind UDP socket");

            udp_client.connect(udp_rcsss_url)
                .await
                .expect("Failed to connect to UDP socket");

            debug!("[{addr}]: udp conn established: {:?} |-> {:?}",
                udp_client.local_addr(), udp_client.peer_addr());

            let udp_tx = Arc::new(udp_client);
            let udp_rx = Arc::clone(&udp_tx);
            (udp_tx, udp_rx)
        };

        // recv from the upstream and send to the downside ws
        let ws_tx_ = ws_tx.clone();
        let udp_rx_task = spawn_and_cancel(cancel.clone(), async move {
            let mut buf = [0; 4096];

            loop {
                match udp_rx.recv_from(&mut buf).await {
                    Ok((0, _)) => {
                        debug!("[{addr}]: udp_rx stopped on 'EOF'");
                        break;
                    },
                    Err(e) => {
                        warn!("[{addr}]: udp_rx breaking due to recv error: {e}");
                        return Err(e.into())
                    },
                    Ok((data, _addr)) => {
                        let data = buf[..data].to_vec();
                        trace!("[{addr}]: Data recv from udp: {:?}...", &data[..32]);
                        let _ = ws_tx_.send(Signal::data(data));
                    }
                }
            }

            Ok(())
        });

        // recv from the ws and send to the upstream udp
        let ws_rx_task = spawn_and_cancel(cancel.clone(), async move {
            while let Some(msg) = ws_rx.next().await {
                let sig = match msg {
                    Err(e) => return Err(e.into()),
                    Ok(msg) => Signal::from(msg),
                };
                trace!("[{addr}]: Signal recv from ws: {sig:?}");
                match sig {
                    Signal::Pong(_) => continue,
                    Signal::Ping(timestamp) => {
                        ws_tx.send(Signal::Pong(timestamp)).await?;
                        debug!("[{addr}]: Ping recv, Pong back")
                    }
                    Signal::Data(data) => {
                        udp_tx.send(&data).await?;
                        debug!("[{addr}]: Ws data transfer to udp, len = {}.", data.len())
                    }
                    _ => {
                        ws_tx.send(Signal::Unknown).await?;
                        debug!("[{addr}]: Unknown signal, passing")
                    },
                }
            }

            Ok(())
        });

        clients.insert(addr, (ws_mpsc_task, udp_rx_task, ws_rx_task));
    }
}

async fn split_ws_spawn_mpsc<M, S>(
    ws_stream: WebSocketStream<S>,
    chan_size: usize,
    addr: Option<SocketAddr>,
) -> (
    mpsc::Sender<M>,
    SplitStream<WebSocketStream<S>>,
    JoinHandle<Result<()>>,
)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    M: Into<Message> + Send + 'static,
{
    let (mut bg_ws_tx, ws_rx) = ws_stream.split();
    let (ws_tx, mut bg_ws_rx) = mpsc::channel::<M>(chan_size);
    let bg_ws_tx_task = tokio::spawn(async move {
        trace!("[{addr:?}]: ws background channel(->ws_tx) started");
        while let Some(msg) = bg_ws_rx.recv().await {
            bg_ws_tx.send(msg.into()).await?;
        }
        debug!("[{addr:?}]: ws background channel(->ws_tx) closed");
        Ok(())
    });
    (ws_tx, ws_rx, bg_ws_tx_task)
}

fn spawn_and_cancel<T, E>(
    cancel: CancellationToken,
    future: impl Future<Output = std::result::Result<T, E>> + Send + 'static,
) -> JoinHandle<std::result::Result<T, E>>
where
    T: std::fmt::Debug + Default + Send + 'static,
    E: std::fmt::Debug + Send + 'static,
{
    tokio::spawn(async move {
        select! {
            res = future => {
                debug!("Task finished, calling cancel.");
                cancel.cancel();
                res
            },
            _ = cancel.cancelled() => {
                debug!("Task cancelled due to the cancel token.");
                Ok(T::default())
            },
        }
    })
}

fn _spawn_and_cancel_default<T>(
    cancel: CancellationToken,
    future: impl Future<Output = T> + Send + 'static,
) -> JoinHandle<T>
where
    T: std::fmt::Debug + Default + Send + 'static,
{
    tokio::spawn(async move {
        select! {
            res = future => { cancel.cancel(); res },
            _ = cancel.cancelled() => T::default(),
        }
    })
}
