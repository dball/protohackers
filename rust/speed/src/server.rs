use std::{collections::BTreeSet, io, time::Duration};

use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot},
    time::{self, Interval},
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
                    // "Help the rust type inferencer out" ?
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

async fn send_error(mut conn: Connection<'_>, msg: &str) -> Result<(), io::Error> {
    conn.write_message(&Message::Error(msg.to_string())).await?;
    Ok(())
}

// TODO is this goofy or good? Feels like it'll call on every select poll
// even in the none case, but really, if it's none, we don't even want to
// participate in the select.
async fn maybe_tick(interval: &mut Option<Interval>) -> Option<()> {
    match interval {
        Some(interval) => {
            interval.tick().await;
            Some(())
        }
        None => None,
    }
}

async fn handle(mut socket: TcpStream, tx: mpsc::Sender<ServerCommand>) -> Result<(), io::Error> {
    let mut heartbeat = None;
    let mut conn = Connection::new(&mut socket);
    let mut kind = ConnKind::Unknown;
    loop {
        tokio::select! {
            msg = conn.read_message() => {
                // TODO do we send_error on error?
                let msg = msg?;
                if let Message::WantHeartbeat(duration) = msg {
                    if heartbeat.is_some() {
                        return send_error(conn, "already beating").await;
                    }
                    heartbeat = Some(time::interval(duration));
                    continue;
                }
                match kind {
                    ConnKind::Unknown => match msg {
                        Message::IAmCamera(camera) => {
                            kind = ConnKind::Camera(camera);
                        }
                        Message::IAmDispatcher(dispatcher) => {
                            kind = ConnKind::Dispatcher(dispatcher);
                        }
                        _ => {
                            return send_error(conn, "unidentified").await;
                        }
                    },
                    ConnKind::Camera(camera) => match msg {
                        Message::Plate(plate, timestamp) => {
                            let cmd = ServerCommand::RecordPlate(camera, plate, timestamp);
                            if tx.send(cmd).await.is_err() {
                                eprintln!("dropped plate record");
                            }
                        }
                        _ => {
                            return send_error(conn, "invalid camera message").await;
                        }
                    },
                    ConnKind::Dispatcher(_) => {
                        return send_error(conn, "invalid dispatcher message").await;
                    }
                }
            }
            Some(_) = maybe_tick(&mut heartbeat) => {
                conn.write_message(&Message::Heartbeat).await?;
            }
        }
    }
}
