pub mod person;
pub mod room;

use person::Person;
use room::Room;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() {
    let room = room::Room::new();
    let listener = TcpListener::bind("0.0.0.0:9000").await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let room2 = room.clone();
        tokio::spawn(async move {
            match process(socket, room2).await {
                Ok(_) => eprintln!("conn.completed"),
                Err(e) => eprintln!("conn.errored {:?}", e),
            }
        });
    }
}

async fn process(mut socket: TcpStream, room: Room) -> io::Result<()> {
    let (reader, mut writer) = socket.split();
    let mut bufreader = io::BufReader::new(reader);
    let mut buffer = Vec::with_capacity(1024);
    writer
        .write(&format!("What is your name?\n").into_bytes())
        .await?;
    if bufreader.read_until(b'\n', &mut buffer).await? == 0 {
        return Ok(());
    }
    buffer.pop();
    let name = String::from_utf8_lossy(&buffer[..]).to_string();
    buffer.clear();
    eprintln!("got name {}", name);
    match Person::new(name) {
        Some(person) => match room.enter(person.clone()).await {
            Some((greeting, mut endpoint)) => {
                eprintln!("entered {}", greeting);
                writer.write(&greeting.into_bytes()).await?;
                loop {
                    tokio::select! {
                        receipt = endpoint.rx.recv() => {
                            match receipt {
                                Some(msg) => {
                                    writer.write(&msg.into_bytes()).await?;
                                },
                                None => {
                                    break;
                                },
                            }
                        },
                        result = bufreader.read_until(b'\n', &mut buffer) => {
                            match result {
                                Ok(0) => {
                                    room.leave(person.clone()).await;
                                    break;
                                },
                                Ok(_) => {
                                    let msg = String::from_utf8_lossy(&buffer[..]).to_string();
                                    eprintln!("read {} from socket", msg);
                                    buffer.clear();
                                    if endpoint.tx.send(msg).await.is_err() {
                                        break;
                                    }
                                },
                                Err(_) => {
                                    room.leave(person.clone()).await;
                                    break;
                                },
                            }
                        },
                    }
                }
            }
            None => {
                eprintln!("no love in the room");
            }
        },
        None => {}
    }
    socket.shutdown().await?;
    Ok(())
}
