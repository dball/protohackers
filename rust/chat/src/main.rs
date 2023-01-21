pub mod person;
pub mod room;
pub mod room2;

use person::Person;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use room::Room;

#[tokio::main]
async fn main() {
    let room = room::Room::new();
    eprintln!("initial room is closed {}", room.cmd_sender.is_closed());
    let listener = TcpListener::bind("0.0.0.0:9000").await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let room2 = room.clone();
        tokio::spawn(async move {
            eprintln!("conn.accepted room closed {}", room2.cmd_sender.is_closed());
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
    writer.write(&format!("What is your name?\n").into_bytes()).await?;
    if bufreader.read_until(b'\n', &mut buffer).await? == 0 {
        return Ok(());
    }
    buffer.pop();
    let name = String::from_utf8_lossy(&buffer[..]).to_string();
    eprintln!("got name {}", name);
    match Person::new(name) {
        Some(person) => {
            eprintln!("got person {:?} room is closed {}", person, room.cmd_sender.is_closed());
            match room.enter(person.clone()).await {
                Some((greeting, mut endpoint)) => {
                    eprintln!("entered {}", greeting);
                    writer.write(&greeting.into_bytes()).await?;
                    loop {
                        tokio::select! {
                            receipt = endpoint.receiver.recv() => {
                                match receipt {
                                    Ok(msg) => {
                                        writer.write(&msg.into_bytes()).await?;
                                    },
                                    Err(e) => {
                                        eprintln!("received error from endpoint {}", e);
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
                                        if endpoint.sender.send(msg).await.is_err() {
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
                },
                None => {
                    eprintln!("no love in the room");
                },
            }
        },
        None => {},
    }
    socket.shutdown().await?;
    Ok(())
}
