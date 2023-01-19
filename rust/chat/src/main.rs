pub mod room;

use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

use crate::room::Room;

#[tokio::main]
async fn main() {
    let room = Room::new();
    let listener = TcpListener::bind("0.0.0.0:9000").await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let conversation = room.enter().await;
        tokio::spawn(async move {
            eprintln!("conn.accepted");
            match process(socket).await {
                Ok(_) => eprintln!("conn.completed"),
                Err(e) => eprintln!("conn.errored {:?}", e),
            }
        });
    }
}

async fn process(mut socket: TcpStream) -> io::Result<()> {
    let (reader, mut writer) = socket.split();
    let mut buffer = Vec::with_capacity(1024);
    let mut bufreader = io::BufReader::new(reader);
    loop {
        match bufreader.read_until(b'\n', &mut buffer).await {
            Ok(0) => {
                break;
            },
            Ok(n) => {
                
            },
            Err(e) => {
                return Err(e)
            },
        }
    }
    socket.shutdown().await?;
    Ok(())
}