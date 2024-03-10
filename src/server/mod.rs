use std::{io, net::SocketAddr, sync::Arc};

use tokio::{
    io::BufReader,
    net::{
        tcp::{ReadHalf, WriteHalf},
        TcpListener, TcpStream, UdpSocket,
    },
    sync::Mutex,
};

use crate::{
    client::parser,
    common::{
        config::Config,
        message::{self, Message, Protocol, Request, Response},
        communication::*
    },
};

mod state;
use state::State;

use self::state::Peer;

pub async fn run(config: Config) -> io::Result<()> {
    let Config { tcp, .. } = config;

    let listener = TcpListener::bind(tcp).await?;

    let state = Arc::new(Mutex::new(State::new().await?));

    loop {
        let udp = UdpSocket::bind("0.0.0.0:0").await?;

        let state = state.clone();
        let (stream, address) = listener.accept().await?;

        tokio::spawn(async move {
            let _ignore = process(state, stream, udp, address).await;
        });
    }
}

async fn get_user_info_and_respond(
    reader: &mut BufReader<ReadHalf<'_>>,
    writer: &mut WriteHalf<'_>,
    local_udp: SocketAddr
) -> io::Result<(String, SocketAddr)> {
    let request = receive_tcp::<Request>(reader).await;

    match request {
        Ok(Request::SignIn { name, udp }) => {
            send_tcp(writer, Response::Ok(local_udp)).await?;
            Ok((name, udp))
        }
        Err(reason) => {
            let err = io::Error::new(
                io::ErrorKind::InvalidInput, 
                reason
            );

            let response = Response::Error(message::Error::InvalidName);
            send_tcp(writer, response).await?;
            Err(err)
        },
        _ => Err(io::Error::new(
            io::ErrorKind::ConnectionAborted,
            "Invalid handshake request format"
        ))
    }
}

async fn disconnect_and_remove(state: Arc<Mutex<State>>, mut user: Peer<'_>, address: &SocketAddr) {
    state.lock().await.remove(&user.name, address);
    user.internal_rx.close();
}

async fn send_internally(state: &Mutex<State>, message: Message) -> io::Result<()> {
    state
        .lock()
        .await
        .send(message)
        .await
        .map_err(|reason| io::Error::new(
            io::ErrorKind::BrokenPipe,
            reason
        ))
}

async fn broadcast_internally(state: &Mutex<State>, message: Message) -> io::Result<()> {
    state
        .lock()
        .await
        .broadcast(message)
        .await
        .map_err(|reason| io::Error::new(
            io::ErrorKind::BrokenPipe,
            reason
        ))
}

async fn send_server_announcement(state: &Mutex<State>, text: &str) -> io::Result<()> {
    let message = Message::new(
        &text, 
        "server", 
        parser::BROADCAST_NAME, 
        Protocol::Tcp
    );

    broadcast_internally(state, message).await
}

async fn process(
    state: Arc<Mutex<State>>,
    mut stream: TcpStream,
    udp: UdpSocket,
    address: SocketAddr,
) -> io::Result<()> {
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    
    let (name, udp_address) = get_user_info_and_respond(
        &mut reader, 
        &mut writer, 
        udp.local_addr()?
    ).await?;

    udp.connect(udp_address).await?;

    let mut user = state
        .lock()
        .await
        .add(reader, writer, udp, address, &name);

    println!("{} @ {} connected", name, address);

    send_server_announcement(
        &state, 
        &format!("{name} has joined the chat")
    ).await?;

    loop {
        tokio::select! {
            Some(msg) = user.internal_rx.recv() => {
                let _ignore = match msg.get_protocol().unwrap() {
                    Protocol::Udp => send_udp(&user.udp, msg).await,
                    Protocol::Tcp => send_tcp(&mut user.writer, msg).await
                };
            }

            Ok(request) = receive_udp::<Request>(&user.udp) => {
                let message = request.to_message(&name).unwrap();
                let _ignore = match message.is_broadcast() {
                    true => broadcast_internally(&state, message).await,
                    false => send_internally(&state, message).await
                };
            }

            result = receive_tcp::<Request>(&mut user.reader) => match result {
                Ok(Request::SignOut) | Err(_) => break,
                Ok(request) => {
                    let message = request.to_message(&name).unwrap();
                    let _ignore = match message.is_broadcast() {
                        true => broadcast_internally(&state, message).await,
                        false => send_internally(&state, message).await
                    };
                }
            }
        }
    }

    send_server_announcement(
        &state, 
        &format!("{name} has left the chat")
    ).await?;

    disconnect_and_remove(state, user, &address).await;

    println!("{} @ {} disconnected", name, address);

    Ok(())
}
