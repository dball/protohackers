use std::{collections::BTreeMap, net::SocketAddr};

use tokio::{
    net::UdpSocket,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
};

use crate::packet::{parse_message, Message, Session};

pub async fn listen(addr: &str) -> anyhow::Result<()> {
    let mut sessions: BTreeMap<Session, UnboundedSender<Message>> = Default::default();
    loop {
        let socket = UdpSocket::bind(addr).await?;
        let mut buf: Vec<u8> = Vec::with_capacity(1000);
        let (len, addr) = socket.recv_from(&mut buf).await?;
        if len == 1000 {
            // Illegal packet, too large.
            continue;
        }
        let message = parse_message(buf);
        if message.is_err() {
            // Illegal message.
            continue;
        }
        let message = message.unwrap();
        match message {
            Message::Connect(session) => {
                let tx = if let Some(tx) = sessions.get(&session) {
                    (*tx).clone()
                } else {
                    let (tx, rx) = unbounded_channel();
                    sessions.insert(session, tx.clone());
                    tx
                };
                tx.send(message);
                continue;
            }
            Message::Close(session) => {}
            Message::Data(data) => {}
            Message::Ack(session, position) => {}
        }
    }
}

async fn handle(addr: SocketAddr, rx: UnboundedReceiver<Message>) {}
