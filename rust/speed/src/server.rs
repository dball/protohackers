use std::io;

use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot},
    time::{self, Interval},
};

use crate::{
    connection::{Connection, Message},
    domain::{Camera, Dispatcher, Plate, Region, Ticket, Timestamp},
};

pub struct Server {
    region: Region,
}

// TODO missing 0QNP from tickets issued... and 553 other cars heh

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
                Ok((socket, _)) = listener.accept() => {
                    let tx = tx.clone();
                    tokio::spawn(async move { handle(socket, tx).await; });
                }
                Some(cmd) = rx.recv() => {
                    match cmd {
                        ServerCommand::RecordPlate(camera, plate, timestamp) => {
                            self.region.record_plate(camera, plate, timestamp);
                        }
                        ServerCommand::RegisterDispatcher(dispatcher, tx) => {
                            tx.send(self.region.register_dispatcher(dispatcher));
                        }
                    }
                }
            };
        }
    }
}

enum ServerCommand {
    RecordPlate(Camera, Plate, Timestamp),
    RegisterDispatcher(Dispatcher, oneshot::Sender<mpsc::Receiver<Ticket>>),
}

async fn send_error(mut conn: Connection<'_>, msg: &str) -> Result<(), io::Error> {
    conn.write_message(&Message::Error(msg.to_string())).await?;
    Ok(())
}

// TODO is this goofy or good? Feels like it'll call on every select poll
// even in the none case, but really, if it's none, we don't even want to
// participate in the select.
async fn maybe_tick(interval: &mut Option<Option<Interval>>) -> Option<()> {
    match interval {
        Some(Some(interval)) => {
            interval.tick().await;
            Some(())
        }
        _ => None,
    }
}

async fn handle(mut socket: TcpStream, tx: mpsc::Sender<ServerCommand>) -> Result<(), io::Error> {
    let mut heartbeat: Option<Option<Interval>> = None;
    let mut conn = Connection::new(&mut socket);
    loop {
        tokio::select! {
            msg = conn.read_message() => {
                match msg {
                    Ok(Message::WantHeartbeat(duration)) => {
                        if heartbeat.is_some() {
                            return send_error(conn, "already beating").await;
                        }
                        if let Some(duration) = duration {
                            heartbeat = Some(Some(time::interval(duration)));
                        } else {
                            heartbeat = Some(None);
                        }
                    }
                    Ok(Message::IAmCamera(camera)) => {
                        return handle_camera(conn, tx, camera, heartbeat).await;
                    }
                    Ok(Message::IAmDispatcher(dispatcher)) => {
                        return handle_dispatcher(conn, tx, dispatcher, heartbeat).await;
                    }
                    Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {},
                    _ => {
                        return send_error(conn, "invalid message").await;
                    }
                }
            }
            Some(_) = maybe_tick(&mut heartbeat) => {
                conn.write_message(&Message::Heartbeat).await?;
            }
        }
    }
}

async fn handle_camera(
    mut conn: Connection<'_>,
    tx: mpsc::Sender<ServerCommand>,
    camera: Camera,
    mut heartbeat: Option<Option<Interval>>,
) -> Result<(), io::Error> {
    loop {
        tokio::select! {
            msg = conn.read_message() => {
                match msg {
                    Ok(Message::WantHeartbeat(duration)) => {
                        if heartbeat.is_some() {
                            return send_error(conn, "already beating").await;
                        }
                        if let Some(duration) = duration {
                            heartbeat = Some(Some(time::interval(duration)));
                        } else {
                            heartbeat = Some(None);
                        }
                    }
                    Ok(Message::Plate(plate, timestamp)) => {
                        let cmd = ServerCommand::RecordPlate(camera, plate, timestamp);
                        if tx.send(cmd).await.is_err() {
                            eprintln!("dropped plate record");
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {},
                    _ => {
                        eprintln!("invalid camera message {:?}", msg);
                        return send_error(conn, "invalid camera message").await;
                    }
                }
            }
            Some(_) = maybe_tick(&mut heartbeat) => {
                conn.write_message(&Message::Heartbeat).await?;
            }
        }
    }
}

async fn handle_dispatcher(
    mut conn: Connection<'_>,
    cmd_tx: mpsc::Sender<ServerCommand>,
    dispatcher: Dispatcher,
    mut heartbeat: Option<Option<Interval>>,
) -> Result<(), io::Error> {
    let (tx, rx) = oneshot::channel();
    let cmd = ServerCommand::RegisterDispatcher(dispatcher, tx);
    if cmd_tx.send(cmd).await.is_err() {
        eprintln!("failed to register dispatcher");
        return Ok(());
    }
    let ticket_rx = rx.await;
    if ticket_rx.is_err() {
        eprintln!("failed to receive dispatcher ticket channel");
        return Ok(());
    }
    let mut ticket_rx = ticket_rx.unwrap();
    loop {
        tokio::select! {
            ticket = ticket_rx.recv() => {
                if ticket.is_none() {
                    eprintln!("ticket channel closed");
                    return Ok(());
                }
                let ticket = ticket.unwrap();
                conn.write_message(&Message::Ticket(ticket)).await?
            }
            msg = conn.read_message() => {
                match msg {
                    Ok(Message::WantHeartbeat(duration)) => {
                        if heartbeat.is_some() {
                            return send_error(conn, "already beating").await;
                        }
                        if let Some(duration) = duration {
                            heartbeat = Some(Some(time::interval(duration)));
                        } else {
                            heartbeat = Some(None);
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {},
                    _ => {
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
