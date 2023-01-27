use std::{collections::BTreeSet, io};

use tokio::{
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{self, Sender},
        oneshot,
    },
};

use crate::{
    connection::Connection,
    domain::{Camera, Dispatcher, Message, Plate, Region, Ticket, Timestamp},
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
        let (tx, mut rx) = mpsc::channel::<ServerCommand>(16);
        let listener = TcpListener::bind("0.0.0.0:9000").await?;
        loop {
            tokio::select! {
                _ = async {
                    loop {
                        let (socket, _) = listener.accept().await?;
                        let tx = tx.clone();
                        tokio::spawn(async move { handle(socket, tx); });
                    }
                    // "Help the rust type inference out" ?
                    Ok::<_, io::Error>(())
                } => {},
                Some(cmd) = rx.recv() => {
                    match cmd {
                        ServerCommand::RecordPlate(camera, plate, timestamp) => {
                            self.region.record_plate(camera, plate, timestamp);
                        }
                        ServerCommand::IssueTicket(dispatcher, tx) => {
                            if tx.send(self.region.issue_ticket(dispatcher)).is_err() {
                                eprintln!("ticket dropped");
                            }
                        }
                    }
                }
            };
        }
    }
}

enum ServerCommand {
    RecordPlate(Camera, Plate, Timestamp),
    IssueTicket(Dispatcher, oneshot::Sender<Option<Ticket>>),
}

enum ConnKind {
    Unknown,
    Camera(Camera),
    Dispatcher(Dispatcher),
}

async fn handle(mut socket: TcpStream, tx: Sender<ServerCommand>) -> Result<(), io::Error> {
    let mut heartbeat_interval = None;
    let mut conn = Connection::new(&mut socket);
    let mut kind = ConnKind::Unknown;
    loop {
        todo!("select across read_message, heartbeat clock, and a ticket dispatch clock");
        // TODO when the clocks are empty, can we use None in the select! form or do we
        // need to use futures::future::OptionFuture ?
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
            todo!("build a clock stream");
            continue;
        }
        match kind {
            ConnKind::Unknown => match msg {
                Message::IAmCamera(camera) => {
                    kind = ConnKind::Camera(camera);
                }
                Message::IAmDispatcher { numroads, roads } => {
                    // TODO should this be a from/into relation between the msg and the struct?
                    let mut droads = BTreeSet::new();
                    roads.iter().for_each(|road| {
                        droads.insert(*road);
                    });
                    let dispatcher = Dispatcher { roads: droads };
                    kind = ConnKind::Dispatcher(dispatcher);
                    todo!("register the dispatcher locally");
                }
                _ => {
                    conn.write_message(&Message::Error {
                        msg: "unidentified".to_string(),
                    })
                    .await?;
                    return Ok(());
                }
            },
            ConnKind::Camera(camera) => match msg {
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
            ConnKind::Dispatcher(dispatcher) => {
                conn.write_message(&Message::Error {
                    msg: "invalid dispatcher message".to_string(),
                })
                .await?;
                return Ok(());
            }
        }
    }
}
