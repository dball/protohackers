use lazy_static::lazy_static;
use regex::Regex;
use std::collections::BTreeMap;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::oneshot;
use tokio::sync::broadcast;

enum Command {
    Enter { res: oneshot::Sender<Pair> },
}

#[derive(Debug)]
pub struct Room {
    sender: Sender<Command>,
}

pub struct Pair {
    sender: Sender<Message>,
    receiver: Receiver<Message>,
}

impl Room {
    pub fn new() -> Self {
        let (cmd_sender, mut cmd_receiver) = channel::<Command>(1);
        let (broadcast_sender, mut broadcast_receiver) = broadcast::channel::<Message>(16);
        tokio::spawn(async move {
            loop {
                match broadcast_receiver.recv().await {
                    Ok(msg) => {
                        eprintln!("room.broadcast {:?}", msg);
                        continue;
                    },
                    Err(e) => {
                        break;
                    }
                }
            }
        });
        let manager = tokio::spawn(async move {
            loop {
                match cmd_receiver.recv().await {
                    Some(cmd) => match cmd {
                        Command::Enter { res } => {
                            let (their_tx, our_rx) = channel::<Message>(1);
                            let (our_tx, their_rx) = channel::<Message>(1);
                            let mut ours = Pair { sender: our_tx, receiver: their_rx };
                            let theirs = Pair { sender: their_tx, receiver: our_rx };
                            if res.send(theirs).is_ok() {
                                tokio::spawn(async move {
                                    if let Some(person) = match ours.receiver.recv().await {
                                        Some(msg) => {
                                            Person::new(msg.contents)
                                        },
                                        None => {
                                            None
                                        },
                                    } {
                                        if ours.sender.send(Message::new("joined".to_owned())).await.is_ok() {

                                        }
                                    }

                                });
                            }
                        }
                    },
                    None => {
                        break;
                    }
                }
            }
        });
        Self { sender: cmd_sender }
    }

    /*
    pub async fn enter(&mut self) -> Option<Conversation> {
        let (sender, receiver) = oneshot::channel();
        let cmd = Command::Enter { res: sender };
        if self.sender.send(cmd).await.is_ok() {
            match receiver.await {
                Ok(conv) => Some(conv),
                Err(_) => None,
            }
        } else {
            None
        }
    }
    */

    /*
    pub fn enter(&mut self) -> Conversation {
        let conv = Conversation::new(1);
        let registration = tokio::spawn(async {
            if let Err(_) = conv.outgoing.sender.send(Message::new("name >".to_owned())).await {
              // couldn't write the name prompt, we're done
              return None;
            }
            let name = conv.incoming.receiver.recv().await;
            match name {
              Some(message) => {
                if let Some(person) = Person::new(message.contents) {
                  // we have a good person
                  let s: String = "* The room contains: ".to_owned();
                  s += &self.occupants.keys().into_iter().map(|person| person.name).collect::<Vec<_>>().join(", ");
                  conv.outgoing.sender.send(Message::new(s)).await;
                  self.occupants.insert(person, conv);
                } else {
                  // they gave us an invalid name, we're done
                  conv.outgoing.sender.send(Message::new("invalid name".to_owned())).await;
                  return None;
                }
              },
              None => {
                // didn't even get a name, we're done
                return None;
              },
            }
            Some(())
        });
        conv
    }
    */
}

#[derive(Debug)]
pub struct Conversation {
    incoming: Connection,
    outgoing: Connection,
}

impl Conversation {
    pub fn new(capacity: usize) -> Self {
        Self {
            incoming: Connection::new(capacity),
            outgoing: Connection::new(capacity),
        }
    }
}

#[derive(Debug)]
pub struct Connection {
    sender: Sender<Message>,
    receiver: Receiver<Message>,
}

impl Connection {
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = channel(capacity);
        Self { sender, receiver }
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    contents: String,
}

impl Message {
    pub fn new(contents: String) -> Self {
        Self { contents }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Person {
    name: String,
}

lazy_static! {
    static ref PERSON_NAME: Regex = Regex::new(r"^[a-zA-Z0-9]{1,64}$").unwrap();
}

impl Person {
    pub fn new(name: String) -> Option<Self> {
        if PERSON_NAME.is_match(&name) {
            Some(Self { name })
        } else {
            None
        }
    }
}
