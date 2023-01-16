use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:9000").await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            match process(socket).await {
                Ok(_) => eprintln!("success"),
                Err(e) => eprintln!("error {}", e),
            }
        });
    }
}

async fn process(mut socket: TcpStream) -> io::Result<()> {
    let (mut reader, _) = socket.split();
    let mut buffer = [0; 1024];
    loop {
        match reader.read(&mut buffer[..]).await {
            Ok(n) if n > 0 => {
                eprintln!("read {}", n)
            }
            Ok(_) => {
                break;
            }
            Err(e) => {
                // Are we supposed to try to shut down the socket?
                return Err(e)
            }
        }
    }
    socket.shutdown().await
}
