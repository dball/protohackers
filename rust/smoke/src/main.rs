use mio::net::{TcpListener};
use mio::{Events, Interest, Poll, Token};
use std::io::{self, Read, Write};
use std::collections::HashMap;
use std::net::SocketAddr;

const SERVER: Token = Token(0);

fn main() -> io::Result<()> {
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(1024);

    let addr = "127.0.0.1:9000".parse::<SocketAddr>().unwrap();
    let mut server = TcpListener::bind(addr)?;

    poll.registry().register(&mut server, SERVER, Interest::READABLE)?;

    let mut current_token = Token(SERVER.0 + 1);
    let mut connections = HashMap::new();

    loop {
        poll.poll(&mut events, None)?;
        for event in events.iter() {
            match event.token() {
                SERVER => loop {
                    let (mut connection, _address) = match server.accept() {
                        Ok((connection, address)) => (connection, address),
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                            break;
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    };
                    let token = next_token(&mut current_token);
                    poll.registry().register(&mut connection, token, Interest::READABLE)?;
                    connections.insert(token, (connection, vec![0; 1024], 0, false));
                },
                token => {
                    match connections.get_mut(&token) {
                        Some((connection, data, read, closed)) => {
                            if event.is_readable() {
                                match connection.read(data) {
                                    Ok(n) => {
                                        if n == 0 {
                                            *closed = true;
                                        } else {
                                            *read = n;
                                        }
                                        poll.registry().reregister(connection, token, Interest::WRITABLE)?;
                                    }
                                    Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => (),
                                    Err(ref err) if err.kind() == io::ErrorKind::Interrupted => (),
                                    Err(err) => return Err(err),
                                }
                            }
                            if event.is_writable() {
                                if *read != 0 {
                                    match connection.write(&data[0..*read]) {
                                        Ok(n) if n == *read => {
                                            poll.registry().reregister(connection, token, Interest::READABLE)?;
                                        },
                                        Ok(_n) => {
                                            panic!("partial write. does zero mean closed? and how do we write a partial vec?");
                                        },
                                        Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => (),
                                        Err(ref err) if err.kind() == io::ErrorKind::Interrupted => (),
                                        Err(err) => return Err(err),
                                    }
                                }
                                if *closed {
                                    poll.registry().deregister(connection)?;
                                    connections.remove(&token);
                                }
                            }
                        },
                        None => {},
                    }
                }
            }
        }
    }
}

fn next_token(current: &mut Token) -> Token {
    let next = current.0;
    current.0 += 1;
    Token(next)
}