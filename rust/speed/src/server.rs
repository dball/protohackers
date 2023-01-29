use std::{io, time::Duration};

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
    loop {
        tokio::select! {
            msg = conn.read_message() => {
                match msg {
                    Ok(Message::WantHeartbeat(duration)) => {
                        if heartbeat.is_some() {
                            return send_error(conn, "already beating").await;
                        }
                        heartbeat = Some(time::interval(duration));
                    }
                    Ok(Message::IAmCamera(camera)) => {
                        return handle_camera(conn, tx, camera, heartbeat).await;
                    }
                    Ok(Message::IAmDispatcher(dispatcher)) => {
                        return handle_dispatcher(conn, tx, dispatcher, heartbeat).await;
                    }
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
    mut heartbeat: Option<Interval>,
) -> Result<(), io::Error> {
    loop {
        tokio::select! {
            msg = conn.read_message() => {
                match msg {
                    Ok(Message::WantHeartbeat(duration)) => {
                        if heartbeat.is_some() {
                            return send_error(conn, "already beating").await;
                        }
                        heartbeat = Some(time::interval(duration));
                    }
                    Ok(Message::Plate(plate, timestamp)) => {
                        let cmd = ServerCommand::RecordPlate(camera, plate, timestamp);
                        if tx.send(cmd).await.is_err() {
                            eprintln!("dropped plate record");
                        }
                    }
                    _ => {
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

async fn maybe_recv<T>(rx: &mut Option<oneshot::Receiver<Option<T>>>) -> Option<T> {
    match rx {
        Some(rx) => rx.await.unwrap(),
        None => None,
    }
}

async fn handle_dispatcher(
    mut conn: Connection<'_>,
    cmd_tx: mpsc::Sender<ServerCommand>,
    dispatcher: Dispatcher,
    mut heartbeat: Option<Interval>,
) -> Result<(), io::Error> {
    let mut issue = time::interval(Duration::from_millis(100));
    let mut ticketer: Option<oneshot::Receiver<Option<Ticket>>> = None;
    loop {
        tokio::select! {
            _ = issue.tick() => {
                let (tx, rx) = oneshot::channel();
                let cmd = ServerCommand::IssueTicket(dispatcher.clone(), tx);
                if cmd_tx.send(cmd).await.is_err() {
                    eprintln!("dropped dispatch request");
                } else {
                    ticketer = Some(rx);
                }
            }
            Some(ticket) = maybe_recv(&mut ticketer) => {
                conn.write_message(&Message::Ticket(ticket)).await?;
                ticketer = None;
            }
            msg = conn.read_message() => {
                match msg {
                    Ok(Message::WantHeartbeat(duration)) => {
                        if heartbeat.is_some() {
                            return send_error(conn, "already beating").await;
                        }
                        heartbeat = Some(time::interval(duration));
                    }
                    _ => {
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
