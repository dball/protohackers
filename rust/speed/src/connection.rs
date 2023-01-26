use std::io;

use crate::message::Message;

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

    async fn write_message(&mut self, message: &Message) -> io::Result<()> {
        match message {
            Message::Error { msg } => {
                self.writer.write_u8(0x10).await?;
                // write_string
                self.writer.write_u8(msg.len().try_into().unwrap()).await?;
                self.writer.write_all(msg.as_bytes()).await?;
            }
            Message::Ticket {
                plate,
                road,
                mile1,
                timestamp1,
                mile2,
                timestamp2,
                speed,
            } => {
                self.writer.write_u8(0x21).await?;
                self.writer
                    .write_u8(plate.len().try_into().unwrap())
                    .await?;
                self.writer.write_all(plate.as_bytes()).await?;
                self.writer.write_u16(*road).await?;
                self.writer.write_u16(*mile1).await?;
                self.writer.write_u32(*timestamp1).await?;
                self.writer.write_u16(*mile2).await?;
                self.writer.write_u32(*timestamp2).await?;
                self.writer.write_u16(*speed).await?;
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

    async fn read_message(&mut self) -> io::Result<Message> {
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
                        Ok(Message::Plate { plate, timestamp })
                    }
                    Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidData, e)),
                }
            }
            0x40 => {
                let interval = self.reader.read_u32().await?;
                Ok(Message::WantHeartbeat { interval })
            }
            0x80 => {
                let road = self.reader.read_u16().await?;
                let mile = self.reader.read_u16().await?;
                let limit = self.reader.read_u16().await?;
                Ok(Message::IAmCamera { road, mile, limit })
            }
            0x81 => {
                let numroads = self.reader.read_u8().await?;
                let mut roads: Vec<u16> = Vec::with_capacity(numroads.into());
                for _ in 0..numroads {
                    roads.push(self.reader.read_u16().await?);
                }
                Ok(Message::IAmDispatcher { numroads, roads })
            }
            _ => Err(io::Error::from(io::ErrorKind::Unsupported)),
        }
    }
}
