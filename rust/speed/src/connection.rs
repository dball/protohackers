use std::{collections::BTreeSet, io, time::Duration};

use crate::domain::{Camera, Dispatcher, Message, Road};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{
        tcp::{ReadHalf, WriteHalf},
        TcpStream,
    },
};

pub struct Connection<'a> {
    reader: BufReader<ReadHalf<'a>>,
    writer: BufWriter<WriteHalf<'a>>,
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
        Ok(())
    }

    pub async fn read_message(&mut self) -> io::Result<Message> {
        match self.reader.read_u8().await? {
            0x20 => {
                let len: usize = self.reader.read_u8().await?.into();
                let mut buf: Vec<u8> = Vec::with_capacity(len);
                let n = self.reader.read(&mut buf[..]).await?;
                if n != len {
                    return Err(io::Error::from(io::ErrorKind::InvalidData));
                }
                match String::from_utf8(buf) {
                    Ok(plate) => {
                        let timestamp = self.reader.read_u32().await?;
                        Ok(Message::Plate(plate, timestamp))
                    }
                    Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidData, e)),
                }
            }
            0x40 => {
                let deciseconds = self.reader.read_u32().await?;
                let duration = Duration::from_micros((deciseconds * 10).into());
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
