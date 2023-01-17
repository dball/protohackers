use serde::{Serialize, Deserialize};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

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
            Ok(0) if buffer.is_empty() => {
                eprintln!("client closed");
                break
            },
            Ok(0) => {
                eprintln!("client closed with partial message!");
                writer.write_all(b"{\"error\": true}\n").await.unwrap();
                break;
            }
            Ok(_) if buffer.last() != Some(&b'\n') => {
                eprintln!("read_until must have been interrupted...?");
                continue
            }
            Ok(_) => {
                buffer.pop();
                match serde_json::from_slice::<Request>(&buffer) {
                    Ok(req) if req.method == "isPrime" => {
                        eprintln!("read isPrime message! {}", req.number);
                        let mut prime = false;
                        if req.number.is_finite() && req.number > 0.0 && req.number.fract() == 0.0 && req.number < u64::MAX as f64 {
                            prime = primes::is_prime(req.number as u64);
                        }
                        let res = Response { method: req.method, prime };
                        let mut data = serde_json::to_vec(&res).unwrap();
                        data.push(10);
                        if let Err(e) = writer.write_all(&data[..]).await { 
                            eprintln!("write error {:?}", e);
                            // Close the socket?
                            return Err(e);
                        }
                        eprintln!("wrote response {}", prime);
                        writer.flush().await?
                    },
                    _ => {
                        eprintln!("read some busted message");
                        writer.write_all(b"{\"error\": true}\n").await.unwrap();
                        break
                    },
                }
                buffer.clear();
            }
            Err(e) => {
                eprintln!("read error {:?}", e);
                // Close the socket?
                return Err(e)
            }
        }
    }
    eprintln!("closing socket");
    socket.shutdown().await?;
    eprintln!("closed socket");
    Ok(())
}