pub mod person;
pub mod room;
pub mod room;

use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() {
    let room = room::Room::new();
    let listener = TcpListener::bind("0.0.0.0:9000").await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let ours = room.enter().await;
        tokio::spawn(async move {
            eprintln!("conn.accepted");
            match process(socket, ours).await {
                Ok(_) => eprintln!("conn.completed"),
                Err(e) => eprintln!("conn.errored {:?}", e),
            }
        });
    }
}

async fn process(mut socket: TcpStream, ours: Option<room::Pair>) -> io::Result<()> {
    if let Some(mut ours) = ours {
        let (reader, mut writer) = socket.split();
        let mut buffer = Vec::with_capacity(1024);
        let mut bufreader = io::BufReader::new(reader);
        loop {
            tokio::select! {
                receipt = ours.receiver.recv() => {
                    match receipt {
                        Some(msg) => {
                            if writer.write(&msg.contents.into_bytes()).await.is_err() {
                                break;
                            }
                            continue;
                        },
                        None => {
                            break;
                        },
                    }
                },
                result = bufreader.read_until(b'\n', &mut buffer) => {
                    match result {
                        Ok(0) => {
                            break;
                        },
                        Ok(_) => {
                            let s = String::from_utf8_lossy(&buffer[..]).to_string();
                            if ours.sender.send(room::Message::new(s)).await.is_err() {
                                break;
                            }
                        },
                        Err(e) => {
                            return Err(e);
                        },
                    }
                },
            }
        }
    }
    socket.shutdown().await?;
    Ok(())
}
