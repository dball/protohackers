use std::collections::BTreeSet;

use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::person::Person;

#[derive(Debug)]
pub enum Command {
    Join {
        person: Person,
        res: oneshot::Sender<(Message, Endpoint)>,
    },
    Leave {
        person: Person,
    },
}

pub type Message = String;

#[derive(Debug)]
pub struct Endpoint {
    pub sender: mpsc::Sender<Message>,
    pub receiver: broadcast::Receiver<Message>,
}

#[derive(Debug, Clone)]
pub struct Room {
    pub cmd_sender: mpsc::Sender<Command>,
}

impl Room {
    pub fn new() -> Self {
        eprintln!("constructing a room fr fr");
        let (cmd_sender, cmd_receiver) = mpsc::channel::<Command>(1);
        let (broadcast_sender, broadcast_receiver) = broadcast::channel::<Message>(16);
        tokio::spawn(
            Self::log_broadcasts(broadcast_receiver)
        );
        let rcv_cmd_sender = cmd_sender.clone();
        eprintln!("about to spawn receiver fr fr");
        tokio::spawn(
            Self::receive_commands(rcv_cmd_sender, cmd_receiver, broadcast_sender)
        );
        eprintln!("yooo");
        Self { cmd_sender }
    }

    pub async fn enter(&self, person: Person) -> Option<(Message, Endpoint)> {
        let (sender, receiver) = oneshot::channel();
        let cmd = Command::Join {
            person,
            res: sender,
        };
        match self.cmd_sender.send(cmd).await {
            Ok(()) => match receiver.await {
                Ok(receipt) => Some(receipt),
                Err(e) => {
                    eprintln!("enter receive error {}", e);
                    None
                }
            },
            Err(e) => {
                eprintln!("enter error {}", e);
                None
            }
        }
    }

    pub async fn leave(&self, person: Person) {
        let cmd = Command::Leave { person };
        self.cmd_sender.send(cmd).await;
    }

    async fn receive_commands(
        cmd_sender: mpsc::Sender<Command>,
        mut cmd_receiver: mpsc::Receiver<Command>,
        broadcast_sender: broadcast::Sender<Message>,
    ) {
        eprintln!("should be listening to commands wtf");
        let mut people: BTreeSet<Person> = BTreeSet::new();
        loop {
            eprintln!("receiving commands");
            match cmd_receiver.recv().await {
                Some(Command::Join { person, res }) => {
                    let msg = format!("* {} has entered the room\n", person.name);
                    broadcast_sender.send(msg).unwrap();
                    let (personal_sender, personal_receiver) = mpsc::channel(1);
                    let personal_broadcast_receiver = broadcast_sender.subscribe();
                    let endpoint = Endpoint {
                        sender: personal_sender,
                        receiver: personal_broadcast_receiver,
                    };
                    let names = people
                        .iter()
                        .map(|p| p.name.clone())
                        .collect::<Vec<String>>()
                        .join(", ");
                    let msg = format!("* The room contains: {}\n", names);
                    people.insert(person.clone());
                    let personal_broadcast_sender = broadcast_sender.clone();
                    let personal_cmd_sender = cmd_sender.clone();
                    tokio::spawn(
                        Self::receive_messages(
                            person,
                            personal_receiver,
                            personal_cmd_sender,
                            personal_broadcast_sender,
                        )
                    );
                    res.send((msg, endpoint)).unwrap();
                }
                Some(Command::Leave { person }) => {
                    people.remove(&person);
                    let msg = format!("* {} has left the room\n", person.name);
                    broadcast_sender.send(msg).unwrap();
                }
                None => {
                    break;
                }
            }
        }
        eprintln!("done listening to commands");
    }

    async fn receive_messages(
        person: Person,
        mut personal_receiver: mpsc::Receiver<Message>,
        cmd_sender: mpsc::Sender<Command>,
        broadcast_sender: broadcast::Sender<Message>,
    ) {
        loop {
            match personal_receiver.recv().await {
                Some(msg) => {
                    let msg = format!("[{}] {}\n", person.name, msg);
                    broadcast_sender.send(msg);
                    continue;
                }
                None => {
                    cmd_sender.send(Command::Leave { person }).await.unwrap();
                    break;
                }
            }
        }
    }

    async fn log_broadcasts(mut receiver: broadcast::Receiver<Message>) {
        eprintln!("loooooop");
        loop {
            eprintln!("logging broadcasts");
            match receiver.recv().await {
                Ok(msg) => {
                    eprintln!("room.broadcast {:?}", msg);
                    continue;
                }
                Err(_) => {
                    break;
                }
            }
        }
    }
}
