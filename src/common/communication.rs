use std::io;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        tcp::{ReadHalf, WriteHalf},
        UdpSocket
    }
};

use crate::common::message::Encode;

const BUFFER_SIZE: usize = 2048;

pub type Reader<'a> = BufReader<ReadHalf<'a>>;
pub type Writer<'a> = WriteHalf<'a>;

pub async fn send_tcp<T: for<'a> Encode<'a>>(writer: &mut Writer<'_>, content: T) -> io::Result<()> {
    match content.as_bytes() {
        Ok(src) => writer.write_all(&src).await,
        Err(reason) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            reason
        ))
    }
}

pub async fn send_udp<T: for<'a> Encode<'a>>(socket: &UdpSocket, content: T) -> io::Result<()> {
    match content.as_bytes() {
        Ok(src) => socket
            .send(&src)
            .await
            .map(|_| ()),
        Err(reason) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            reason
        ))
    }
}

pub async fn receive_tcp<T: for<'a> Encode<'a>>(reader: &mut Reader<'_>) -> io::Result<T> {
    let mut buffer = Vec::<u8>::with_capacity(BUFFER_SIZE);
    let _ = reader.read_until(b'\n', &mut buffer).await?;

    T::from_bytes(&buffer)
        .map_err(|reason| io::Error::new(
            io::ErrorKind::InvalidData, 
            reason
        ))
}

pub async fn receive_udp<T: for<'a> Encode<'a>>(socket: &UdpSocket) -> io::Result<T> {
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let length = socket.recv(&mut buffer).await?;

    T::from_bytes(&buffer[..length])
        .map_err(|reason| io::Error::new(
            io::ErrorKind::InvalidData, 
            reason
        ))
}