use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use tokio::net::UdpSocket;

enum Command {
    Get { key: Vec<u8>, addr: SocketAddr },
    Set { key: Vec<u8>, value: Vec<u8> },
}

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    let mut data: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();
    let version_key = b"version".to_vec();
    data.insert(version_key.clone(), b"0".to_vec());
    loop {
        let socket = UdpSocket::bind("0.0.0.0:9000").await?;
        match read_command(&socket).await {
            Ok(Some(Command::Get { key, addr })) => {
                if let Some(value) = data.get(&key) {
                    if socket.send_to(&value[..], addr).await.is_err() {}
                }
            }
            Ok(Some(Command::Set { key, value })) if key != version_key => {
                data.insert(key, value);
            }
            _ => (),
        }
    }
}

async fn read_command(socket: &UdpSocket) -> Result<Option<Command>, io::Error> {
    let mut buf: [u8; 1000] = [0; 1000];
    match socket.recv_from(&mut buf).await? {
        (1000, _) => Ok(None),
        (len, addr) => {
            let buf = &mut buf[..len];
            if let Some(i) = buf.iter().position(|b| b == &b'=') {
                let (key, value) = buf.split_at(i);
                let key = key.to_vec();
                let value = value.to_vec();
                Ok(Some(Command::Set { key, value }))
            } else {
                let key = buf.to_vec();
                Ok(Some(Command::Get { key, addr }))
            }
        }
    }
}
