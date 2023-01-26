use std::io;

use crate::message::Message;

use tokio::{
    io::{AsyncWriteExt, BufReader, BufWriter},
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
}
