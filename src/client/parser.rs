use crate::common::message::{Protocol, Request};

pub const BROADCAST_NAME: &'static str = "all";
pub const UDP_MODIFIER: &'static str = "udp";
const SENDER_DELIMITER: char = ':';
const QUIT_COMMAND: &'static str = "quit";

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Send { message: String, receiver: String, protocol: Protocol },
    Quit
}

fn partition(input: &str) -> Option<(String, String)> {
    let mid = input.find(SENDER_DELIMITER)?;
    Some((
        (&input[..mid]).to_owned(), 
        (&input[mid + 1..]).to_owned()
    ))
}

fn cleanup(message: String) -> String {
    message.replace('\n', "")
}

impl Command {
    pub fn from(input: &str) -> Option<Self> {
        if input == QUIT_COMMAND {
            return Some(Self::Quit);
        }

        let (receiver, message) = partition(&input)?;

        if receiver.starts_with(UDP_MODIFIER) {
            let receiver = receiver
                .replace(UDP_MODIFIER, "")
                .strip_prefix(" ")?
                .to_owned();

            Some(Self::Send { receiver, message, protocol: Protocol::Udp })
        } else {
            Some(Self::Send { receiver, message, protocol: Protocol::Tcp })
        }
    }

    pub fn to_request(self) -> Request {
        match self {
            Self::Quit => Request::SignOut,
            Self::Send { message, receiver, protocol } => {
                let message = cleanup(message);

                if &receiver[..] == BROADCAST_NAME {
                    Request::SendAll { message, protocol }
                } else {
                    Request::Send { 
                        receiver, 
                        message, 
                        protocol
                    }
                }
            }
        }
    }
}