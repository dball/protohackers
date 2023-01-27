use std::{
    collections::{BTreeSet, HashMap},
    io,
};

use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
};

use crate::{
    connection::Connection,
    domain::{Camera, Dispatcher, Message, Region},
};

pub struct Server {
    region: Region,
}

impl Server {
    pub fn new() -> Self {
        Self {
            region: Region::new(),
        }
    }

    pub async fn run(&mut self) -> Result<(), io::Error> {
        let listener = TcpListener::bind("0.0.0.0:9000").await?;
        loop {
            let (mut socket, _) = listener.accept().await?;
            tokio::spawn(async move {
                handle(socket);
            });
        }
    }
}

enum Kind {
    Unknown,
    Camera(Camera),
    Dispatcher(Dispatcher),
}

async fn handle(mut socket: TcpStream) -> Result<(), io::Error> {
    let mut heartbeat_interval = None;
    let mut conn = Connection::new(&mut socket);
    let mut kind = Kind::Unknown;
    loop {
        let msg = conn.read_message().await?;
        if let Message::WantHeartbeat { interval } = msg {
            if heartbeat_interval.is_some() {
                conn.write_message(&Message::Error {
                    msg: "already beating".to_string(),
                })
                .await?;
                return Ok(());
            }
            heartbeat_interval = Some(interval);
            todo!("build a clock stream and select across it and read_message in the loop");
            continue;
        }
        match kind {
            Kind::Unknown => match msg {
                Message::IAmCamera(camera) => {
                    kind = Kind::Camera(camera);
                }
                Message::IAmDispatcher { numroads, roads } => {
                    // TODO should this be a from/into relation between the msg and the struct?
                    let mut droads = BTreeSet::new();
                    roads.iter().for_each(|road| {
                        droads.insert(*road);
                    });
                    let dispatcher = Dispatcher { roads: droads };
                    kind = Kind::Dispatcher(dispatcher);
                }
                _ => {
                    conn.write_message(&Message::Error {
                        msg: "unidentified".to_string(),
                    })
                    .await?;
                    return Ok(());
                }
            },
            Kind::Camera(camera) => match msg {
                Message::Plate(plate, timestamp) => {
                    todo!("send plate to the region");
                }
                _ => {
                    conn.write_message(&Message::Error {
                        msg: "invalid camera message".to_string(),
                    })
                    .await?;
                    return Ok(());
                }
            },
            Kind::Dispatcher(dispatcher) => {
                conn.write_message(&Message::Error {
                    msg: "invalid dispatcher message".to_string(),
                })
                .await?;
                return Ok(());
            }
        }
    }
}
