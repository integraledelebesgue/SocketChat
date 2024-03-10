use std::{
    io,
    collections::HashMap,
    net::SocketAddr
};

use tokio::{
    net::{UdpSocket, tcp::{WriteHalf, ReadHalf}},
    sync::mpsc::{self, UnboundedSender, UnboundedReceiver},
    io::BufReader,
};

use bimap::BiMap;

use crate::common::message::{Response, Message};

pub type Sender = UnboundedSender<Response>;
pub type Receiver = UnboundedReceiver<Response>;

pub struct Peer<'a> {
    pub name: String,
    pub reader: BufReader<ReadHalf<'a>>,
    pub writer: WriteHalf<'a>,
    pub udp: UdpSocket,
    pub internal_rx: Receiver,
}

#[allow(dead_code)]
pub struct State {
    pub peers: HashMap<SocketAddr, Sender>,
    names: BiMap<String, SocketAddr>,
    broadcast: UdpSocket,
}

#[derive(Debug, Clone, Copy, Error)]
pub enum SendError {
    UserNotFound,
    InternalChannelFailed,
}

#[allow(dead_code)]
impl State {
    pub async fn new() -> io::Result<Self> {
        let broadcast = UdpSocket::bind("0.0.0.0:0").await?;

        broadcast.set_broadcast(true)?;

        Ok(State {
            peers: HashMap::new(),
            names: BiMap::new(),
            broadcast,
        })
    }

    pub fn add<'a>(
        &mut self, 
        reader: BufReader<ReadHalf<'a>>,
        writer: WriteHalf<'a>,
        udp: UdpSocket,
        address: SocketAddr,
        name: &str
    ) -> Peer<'a> {
        let (internal_tx, internal_rx) = mpsc::unbounded_channel();

        self.peers.insert(address, internal_tx);
        self.names.insert(name.to_owned(), address);

        let name = name.to_owned();

        Peer { name, reader, writer, udp, internal_rx }
    }

    pub fn remove(&mut self, name: &str, address: &SocketAddr) {
        self.peers.remove(address);
        self.names.remove_by_left(name);
    }

    pub async fn send(&mut self, message: Message) -> Result<(), SendError> {
        let receiver = self
            .names
            .get_by_left(message.get_receiver())
            .ok_or(SendError::UserNotFound)?;

        let receiver = self
            .peers
            .get_mut(receiver)
            .ok_or(SendError::UserNotFound)?;

        let message = Response::Message(message);

        receiver
            .send(message)
            .or(Err(SendError::InternalChannelFailed))?;

        Ok(())
    }

    pub async fn broadcast(&mut self, message: Message) -> Result<(), SendError> {
        // let sender = self.names
        //     .get_by_left(message.get_sender())
        //     .ok_or(SendError::UserNotFound)?
        //     .clone();

        let message = Response::Message(message);

        for (_, tx) in self.peers.iter_mut() {
            let _ignore = tx.send(message.clone());
        }

        Ok(())
    }
}
