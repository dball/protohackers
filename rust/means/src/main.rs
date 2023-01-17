use std::io::ErrorKind;

use tokio::{net::{TcpListener, TcpStream, tcp::{ReadHalf, WriteHalf}}, io::{self, AsyncReadExt, AsyncWriteExt, BufReader}};

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
    while let Ok(Some(())) = handle(&mut ledger, &mut bufreader, &mut writer).await {}
    socket.shutdown().await?;
    Ok(())
}

async fn handle(ledger: &mut Ledger, reader: &mut BufReader<ReadHalf<'_>>, writer: &mut WriteHalf<'_>) -> io::Result<Option<()>> {
    match reader.read_u8().await {
        Ok(b'I') => {
            let inst = reader.read_i32().await?;
            let price = reader.read_i32().await?;
            let inserted = ledger.insert(inst, price);
            if !inserted {
                return Ok(None)
            }
        },
        Ok(b'Q') => {
            let min = reader.read_i32().await?;
            let max = reader.read_i32().await?;
            let mut mean = 0;
            if let Some(value) = ledger.mean(min, max) {
                mean = value;
            }
            writer.write_i32(mean).await?;
        },
        Ok(_) => {
            return Ok(None)
        },
        Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
            return Ok(None)
        },
        Err(e) => {
            return Err(e);
        },
    }
    Ok(Some(()))
}