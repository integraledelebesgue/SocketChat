use std::io;

use tokio::{
    io::BufReader,
    net::{
        TcpStream,
        UdpSocket
    },
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
};

use crate::common::{
    config::Config,
    message::{Protocol, Request, Response},
    communication::*
};

async fn login<'a>(
    mut reader: Reader<'a>, 
    mut writer: Writer<'a>,
    udp: UdpSocket,
    name: &str
) -> io::Result<(Reader<'a>, Writer<'a>, UdpSocket)> {
    let local_udp = udp.local_addr()?;
    let request = Request::SignIn { name: name.to_owned(), udp: local_udp };

    send_tcp(&mut writer, request).await?;

    let response = receive_tcp(&mut reader).await?;

    match response {
        Response::Ok(server_udp) => {
            udp.connect(server_udp).await?;
            Ok((reader, writer, udp))
        },
        Response::Error(err) => Err(io::Error::new(
            io::ErrorKind::ConnectionAborted, 
            err
        )),
        Response::Message(_) => Err(io::Error::new(
            io::ErrorKind::ConnectionAborted,
            "Invalid server response"
        ))
    }
}

async fn setup_communication<'a>(
    stream: &'a mut TcpStream
) -> io::Result<(Reader<'a>, Writer<'a>, UdpSocket)> {
    let (reader, writer) = stream.split();
    let reader = BufReader::new(reader);

    let udp = UdpSocket::bind("0.0.0.0:0").await?;

    Ok((reader, writer, udp))
}

type Source = UnboundedReceiver<Request>;
type Sink = UnboundedSender<Response>;

pub async fn run(
    config: Config,
    mut source: Source,
    sink: Sink
) -> io::Result<()> {
    let Config { tcp, name, .. } = config;

    let mut stream = TcpStream::connect(tcp).await?;
    let (reader, writer, udp) = setup_communication(&mut stream).await?;
    let (mut reader, mut writer, udp) = login(
        reader, 
        writer, 
        udp, 
        &name
    ).await?;

    loop {
        tokio::select! {
            request = source.recv() => match request {
                None => break Ok(()),
                Some(request) => match request {
                    Request::Send { protocol: Protocol::Udp, .. } => send_udp(&udp, request).await?,
                    Request::SendAll { protocol: Protocol::Udp, .. } => { /* TODO */ },
                    other => send_tcp(&mut writer, other).await?
                }
            },

            Ok(response) = receive_tcp(&mut reader) => {
                sink.send(response).map_err(|reason| io::Error::new(
                    io::ErrorKind::BrokenPipe, 
                    reason
                ))?;
            },

            Ok(response) = receive_udp(&udp) => {
                sink.send(response).map_err(|reason| io::Error::new(
                    io::ErrorKind::BrokenPipe, 
                    reason
                ))?;
            },

            else => { }
        }
    }
}
