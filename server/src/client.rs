use std::net::SocketAddr;
use tokio::net::UdpSocket;

use crate::error::{Result, Error};

// the actual udp proxy client
#[derive(Debug)]
pub struct ClientSocket {
    peer_addr: SocketAddr,
    socket: UdpSocket,
}

impl ClientSocket {
    async fn establish(host: SocketAddr, peer: SocketAddr) -> Result<Self> {
        let socket = UdpSocket::bind(host).await?;
        socket.connect(peer).await?;

        Ok(ClientSocket {
            peer_addr: peer,
            socket,
        })
    }

    pub fn peer(&self) -> SocketAddr {
        self.peer_addr
    }

    pub async fn change_peer(&mut self, peer: SocketAddr) -> Result<()> {
        self.socket.connect(peer).await?;
        self.peer_addr = peer;
        Ok(())
    }

    pub async fn send(&mut self, buf: &[u8]) -> Result<usize> {
        let len = self.socket.send(buf).await?;
        Ok(len)
    }

    pub async fn recv(&mut self, buf: &mut [u8]) -> Result<usize> {
        let len = self.socket.recv(buf).await?;
        Ok(len)
    }

    pub async fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        let (len, src) = self.socket.recv_from(buf).await?;
        Ok((len, src))
    }
}

#[derive(Debug)]
pub enum SocketState {
    Established,
    Initialized,
    Listening,
    Disconnected,
}

#[derive(Debug)]
pub struct Client {
    alias:  Option<String>,
    host:   SocketAddr,
    socket: Option<ClientSocket>,
    state:  SocketState,
}

impl Client {
    pub fn new(host: SocketAddr, alias: Option<String>) -> Self {
        Client {
            alias,
            host,
            socket: None,
            state:  SocketState::Disconnected,
        }
    }

    async fn connect(&mut self, peer: SocketAddr) -> Result<()> {
        if self.socket.is_some() {
            return Err(Error::Udp("Already connected".to_string()));
        }
        let socket = ClientSocket::establish(self.host, peer).await?;
        self.socket = Some(socket);
        self.state = SocketState::Established;
        Ok(())
    }

    pub fn set_alias(&mut self, alias: String) {
        self.alias = Some(alias);
    }

    pub fn alias(&self) -> &str {
        match &self.alias {
            Some(alias) => alias,
            None => "",
        }
    }

    async fn recv() {
        todo!()
    }

    async fn send() {

    }

}