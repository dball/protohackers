use tokio::sync::mpsc::{Receiver, Sender};

use crate::packet::{Message, Position, Session};

//
pub struct Stream {
    session: Session,
    received: Position,
    sent: Position,
}

impl<'a> Stream {
    pub async fn receive(&mut self, message: Message<'a>) -> anyhow::Result<Message> {
        match message {
            Message::Connect(session) => {
                assert_eq!(self.session, session);
                Ok(Message::Ack(session, 0))
            }
            Message::Close(session) => {
                assert_eq!(self.session, session);
                Ok(Message::Close(session))
            }
            Message::Data(session, position, data) => {
                assert_eq!(self.session, session);
                todo!("something")
            }
            _ => Err(anyhow::anyhow!("nope")),
        }
    }
}
