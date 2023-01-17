use std::io::ErrorKind;

use tokio::{net::{TcpListener, TcpStream, tcp::ReadHalf}, io::{self, AsyncReadExt, AsyncWriteExt}};

use crate::ledger::Ledger;

pub mod ledger;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("0.0.0.0:9000").await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            eprintln!("accepted");
            match process(socket).await {
                Ok(_) => eprintln!("completed normally"),
                Err(e) => eprintln!("completed with error {:?}", e),
            }
        });
    }
}

async fn process(mut socket: TcpStream) -> io::Result<()> {
    let mut ledger = Ledger::new();
    let (reader, mut writer) = socket.split();
    let mut bufreader = io::BufReader::new(reader);
    loop {
        match bufreader.read_u8().await {
            Ok(b'I') => {
                let inst = bufreader.read_i32().await?;
                let price = bufreader.read_i32().await?;
                let inserted = ledger.insert(inst, price);
                if !inserted {
                    break;
                }
            },
            Ok(b'Q') => {
                let min = bufreader.read_i32().await?;
                let max = bufreader.read_i32().await?;
                let mut mean = 0;
                match ledger.mean(min, max) {
                    Some(value) => {
                        mean = value;
                    },
                    None => (),
                }
                writer.write_i32(mean).await?;
            },
            Ok(_) => {
                break;
            },
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                break;
            },
            Err(e) => {
                return Err(e);
            },
        }
    }
    socket.shutdown().await?;
    Ok(())
}