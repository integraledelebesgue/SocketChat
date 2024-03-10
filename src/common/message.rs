use std::{fmt::Display, net::SocketAddr};

use serde::{Serialize, Deserialize};
use postcard;

use crate::client::parser;

pub trait Encode<'a>: Serialize + Deserialize<'a> + Clone {
    fn from_bytes(bytes: &'a [u8]) -> Result<Self, postcard::Error> {
        postcard::from_bytes::<Self>(bytes)
    }

    fn as_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        let mut bytes = postcard::to_extend(&self, Vec::new())?;
        bytes.push(b'\n');
        Ok(bytes)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    SignIn { name: String, udp: SocketAddr },
    SignOut,
    Send { receiver: String, message: String, protocol: Protocol },
    SendAll { message: String, protocol: Protocol }
}

impl<'a> Encode<'a> for Request {}

impl Request {
    pub fn to_message(self, sender: &str) -> Option<Message> {
        match self {
            Request::Send { receiver, message, protocol } => Some(Message::new(
                &message, 
                sender, 
                &receiver, 
                protocol
            )),
            Request::SendAll { message, protocol } => Some(Message::new(
                &message,
                sender,
                parser::BROADCAST_NAME,
                protocol
            )),
            _ => None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Protocol {
    Tcp,
    Udp
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    message: String,
    sender: String,
    receiver: String,
    pub protocol: Protocol
}

#[allow(dead_code)]
impl Message {
    pub fn new(message: &str, sender: &str, receiver: &str, protocol: Protocol) -> Self {
        let message = message.to_owned();
        let sender = sender.to_owned();
        let receiver = receiver.to_owned();

        Message { message, sender, receiver, protocol }
    }

    pub fn is_broadcast(&self) -> bool {
        self.receiver == parser::BROADCAST_NAME
    }

    pub fn get_sender(&self) -> &str {
        &self.sender
    }

    pub fn get_receiver(&self) -> &str {
        &self.receiver
    }
}

impl<'a> Encode<'a> for Message {}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.receiver[..] {
            parser::BROADCAST_NAME => write!(f, "(all) [{}]: {}", self.sender, self.message),
            _ => write!(f, "[{}]: {}", self.sender, self.message)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Error, Serialize, Deserialize)]
pub enum Error {
    InvalidName,
    InvalidServerResponse,

}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Response {
    Ok(SocketAddr),
    Message(Message),
    Error(Error)
}

impl<'a> Encode<'a> for Response {}

impl Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Response::Ok(addr) => { write!(f, "[server] Logged in; server udp: {addr}") },
            Response::Error(reason) => { write!(f, "[server] Error: {reason}") },
            Response::Message(msg) => { write!(f, "{msg}") }
        }
    }
}

impl Response {
    pub fn get_protocol(&self) -> Option<Protocol> {
        if let Self::Message(Message { protocol, .. }) = self {
            Some(*protocol)
        } else {
            None
        }
    }
}
