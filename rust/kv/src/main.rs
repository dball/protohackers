use std::collections::HashMap;
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() {
    let mut data: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();
    let version_key = b"version";
    let version = b"version=0";
    loop {
        let mut buf: [u8; 1000] = [0; 1000];
        match UdpSocket::bind("0.0.0.0:9000").await {
            Ok(socket) => match socket.recv_from(&mut buf).await {
                Ok((1000, _)) => (),
                Ok((len, addr)) => {
                    let read = &mut buf[0..len];
                    match read.iter().position(|b| b == &b'=') {
                        Some(i) => {
                            let (k, _) = read.split_at(i);
                            if k != version_key {
                                data.insert(k.to_vec(), read.to_vec());
                            }
                        }
                        None if read == version_key => match socket.send(version).await {
                            Ok(_) => (),
                            Err(_) => (),
                        },
                        None => match data.get(read) {
                            Some(v) => match socket.connect(addr).await {
                                Ok(_) => match socket.send(&v[..]).await {
                                    Ok(_) => (),
                                    Err(_) => (),
                                },
                                Err(_) => (),
                            },
                            None => (),
                        },
                    }
                }
                Err(_) => (),
            },
            Err(_) => (),
        }
    }
}
