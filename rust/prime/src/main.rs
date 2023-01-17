use serde::{Serialize, Deserialize};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("0.0.0.0:9000").await.unwrap();
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

#[derive(Serialize, Deserialize, Debug)]
struct Request {
    method: String,
    number: f64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Response {
    method: String,
    prime: bool,
}

async fn process(mut socket: TcpStream) -> io::Result<()> {
    let (reader, mut writer) = socket.split();
    let mut buffer = Vec::with_capacity(1024);
    let mut bufreader = io::BufReader::new(reader);
    loop {
        match bufreader.read_until(b'\n', &mut buffer).await {
            Ok(0) if buffer.is_empty() => break,
            Ok(0) => {
                writer.write_all(b"{\"error\": true}").await.unwrap();
                break;
            }
            Ok(_) if buffer.last() != Some(&b'\n') => continue,
            Ok(_) => {
                buffer.pop();
                match serde_json::from_slice::<Request>(&buffer) {
                    Ok(req) if req.method == "prime" => {
                        let mut prime = false;
                        if req.number.is_finite() && req.number > 0.0 && req.number.fract() == 0.0 && req.number < u64::MAX as f64 {
                            prime = primes::is_prime(req.number as u64);
                        }
                        let res = Response { method: req.method, prime };
                        let data = serde_json::to_vec(&res).unwrap();
                        if let Err(e) = writer.write_all(&data[..]).await { 
                            eprintln!("write error {:?}", e);
                            // Close the socket?
                            return Err(e);
                        }
                    },
                    _ => {
                        writer.write_all(b"{\"error\": true}").await.unwrap();
                        break
                    },
                }
                buffer.clear();
            }
            Err(e) => {
                // Close the socket?
                return Err(e)
            }
        }
    }
    socket.shutdown().await?;
    Ok(())
}