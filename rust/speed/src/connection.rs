use std::{io, time::Duration};

use crate::domain::{Camera, Dispatcher, Plate, Ticket, Timestamp};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{
        tcp::{ReadHalf, WriteHalf},
        TcpStream,
    },
};

#[derive(Debug)]
pub struct Connection<'a> {
    reader: BufReader<ReadHalf<'a>>,
    writer: BufWriter<WriteHalf<'a>>,
}

// Messages are sent between clients and the server.
//
// The server expects a client to identify its type.
//
// server: {IAmCamera: [Plate]*, IAmDispatcher: []} with one WantHeartbeat allowed at any point
// camera clients expect only heartbeats, dispatchers also expect tickets.
//
// TODO how can we statically encode the code on the cases?
// TODO if we did, would that affect the dispatch speed in reads, writes, etc.?
// TODO how can we encode the direction, and/or the state sequence constraints on the message types
// TODO how can we construct de/serialization code for these declaratively?
#[derive(Debug)]
pub enum Message {
    // server -> client
    Error(String),
    // camera -> server
    Plate(Plate, Timestamp),
    // server -> dispatcher
    Ticket(Ticket),
    // client -> server
    WantHeartbeat(Option<Duration>),
    // server -> client
    Heartbeat,
    // (client->camera) -> server
    IAmCamera(Camera),
    // (client->dispatcher) -> server
    IAmDispatcher(Dispatcher),
}

impl<'a> Connection<'a> {
    pub fn new(socket: &'a mut TcpStream) -> Connection<'a> {
        let (reader, writer) = socket.split();
        let reader = BufReader::new(reader);
        let writer = BufWriter::new(writer);
        Connection { reader, writer }
    }

    pub async fn write_message(&mut self, message: &Message) -> io::Result<()> {
        match message {
            Message::Error(msg) => {
                self.writer.write_u8(0x10).await?;
                // write_string
                self.writer.write_u8(msg.len().try_into().unwrap()).await?;
                self.writer.write_all(msg.as_bytes()).await?;
            }
            Message::Ticket(ticket) => {
                self.writer.write_u8(0x21).await?;
                self.writer
                    .write_u8(ticket.plate.len().try_into().unwrap())
                    .await?;
                self.writer.write_all(ticket.plate.as_bytes()).await?;
                self.writer.write_u16(ticket.road).await?;
                self.writer.write_u16(ticket.mile1).await?;
                self.writer.write_u32(ticket.timestamp1).await?;
                self.writer.write_u16(ticket.mile2).await?;
                self.writer.write_u32(ticket.timestamp2).await?;
                self.writer.write_u16(ticket.speed).await?;
            }
            Message::Heartbeat => {
                self.writer.write_u8(0x41).await?;
            }
            _ => {
                unimplemented!("This is only the server.")
            }
        }
        self.writer.flush().await?;
        Ok(())
    }

    pub async fn read_message(&mut self) -> io::Result<Message> {
        match self.reader.read_u8().await? {
            0x20 => {
                let len: usize = self.reader.read_u8().await?.into();
                let mut buf: Vec<u8> = Vec::with_capacity(len);
                buf.resize(len, 0);
                self.reader.read_exact(&mut buf[..]).await?;
                match String::from_utf8(buf) {
                    Ok(plate) => {
                        let timestamp = self.reader.read_u32().await?;
                        Ok(Message::Plate(plate, timestamp))
                    }
                    Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
                }
            }
            0x40 => {
                let deciseconds = self.reader.read_u32().await?;
                let duration = if deciseconds == 0 {
                    None
                } else {
                    Some(Duration::from_millis((deciseconds * 100).into()))
                };
                Ok(Message::WantHeartbeat(duration))
            }
            0x80 => {
                let road = self.reader.read_u16().await?;
                let mile = self.reader.read_u16().await?;
                let limit = self.reader.read_u16().await?;
                Ok(Message::IAmCamera(Camera { road, mile, limit }))
            }
            0x81 => {
                let numroads = self.reader.read_u8().await?;
                let mut dispatcher: Dispatcher = Default::default();
                for _ in 0..numroads {
                    dispatcher.roads.insert(self.reader.read_u16().await?);
                }
                Ok(Message::IAmDispatcher(dispatcher))
            }
            _ => Err(io::Error::from(io::ErrorKind::Unsupported)),
        }
    }
}
