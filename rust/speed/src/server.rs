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

#[derive(Debug)]
pub struct Server {
    region: Region,
}

impl Server {
    pub fn new() -> Self {
        Self {
            region: Region::new(),
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn run(&mut self) -> Result<(), io::Error> {
        tracing::info!("starting server");
        let (tx, mut rx) = mpsc::channel::<ServerCommand>(16);
        let listener = TcpListener::bind("0.0.0.0:9000").await?;
        loop {
            tokio::select! {
                Ok((socket, _)) = listener.accept() => {
                    let tx = tx.clone();
                    tokio::spawn(async move {
                        if let Err(err) = handle(socket, tx).await {
                            tracing::error!(?err, "handling socket");
                        }
                    });
                }
                Some(cmd) = rx.recv() => {
                    match cmd {
                        ServerCommand::RecordPlate(camera, plate, timestamp) => {
                            self.region.record_plate(camera, plate, timestamp);
                        }
                        ServerCommand::RegisterDispatcher(dispatcher, tx) => {
                            if let Err(err) = tx.send(self.region.register_dispatcher(dispatcher.clone())) {
                                tracing::error!(?err, ?dispatcher, "registering dispatcher");
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
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

#[tracing::instrument(skip_all)]
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

#[tracing::instrument(skip(conn, tx, heartbeat))]
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
                        if let Err(err) = tx.send(cmd).await {
                            tracing::error!(?err, "dropped plate record");
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {},
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

#[tracing::instrument(skip(conn, cmd_tx, heartbeat))]
async fn handle_dispatcher(
    mut conn: Connection<'_>,
    cmd_tx: mpsc::Sender<ServerCommand>,
    dispatcher: Dispatcher,
    mut heartbeat: Option<Option<Interval>>,
) -> Result<(), io::Error> {
    // This copy of the loop is just to handle the case where roads is empty, and therefore
    // the tickets_rx will always be closed/ing because there will be no tickets_tx stored
    // in the road dispatchers collection.
    if dispatcher.roads.is_empty() {
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
    } else {
        let (tx, rx) = oneshot::channel();
        let cmd = ServerCommand::RegisterDispatcher(dispatcher, tx);
        if let Err(err) = cmd_tx.send(cmd).await {
            tracing::error!(?err, "failed to register dispatcher");
            return Ok(());
        }
        let ticket_rx = rx.await;
        if let Err(err) = ticket_rx {
            tracing::error!(?err, "failed to receive dispatcher ticket channel");
            return Ok(());
        }
        let mut ticket_rx = ticket_rx.unwrap();
        loop {
            tokio::select! {
                ticket = ticket_rx.recv() => {
                    if ticket.is_none() {
                        tracing::error!("ticket channel closed");
                        return Ok(());
                    }
                    let ticket = ticket.unwrap();
                    tracing::info!(?ticket, "writing ticket");
                    if let Err(err) = conn.write_message(&Message::Ticket(ticket.clone())).await {
                        tracing::error!(?ticket, ?err, "error writing ticket, closing dispatcher");
                        return Err(err);
                    }
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
}
