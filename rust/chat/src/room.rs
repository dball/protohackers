use std::collections::BTreeMap;

use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::person::Person;

#[derive(Debug)]
pub enum Command {
    Join {
        person: Person,
        res: oneshot::Sender<(Message, Endpoint)>,
    },
    Say {
        person: Person,
        message: Message,
    },
    Leave {
        person: Person,
    },
}

pub type Message = String;

#[derive(Debug)]
pub struct Endpoint {
    pub tx: mpsc::Sender<Message>,
    pub rx: mpsc::Receiver<Message>,
}

#[derive(Debug, Clone)]
pub struct Room {
    pub cmd_tx: mpsc::Sender<Command>,
}

impl Room {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel(1);
        let cmd_tx_clone = cmd_tx.clone();
        tokio::spawn(Self::receive_commands(cmd_tx_clone, cmd_rx));
        Self { cmd_tx }
    }

    pub async fn enter(&self, person: Person) -> Option<(Message, Endpoint)> {
        let (tx, rx) = oneshot::channel();
        let cmd = Command::Join { person, res: tx };
        match self.cmd_tx.send(cmd).await {
            Ok(()) => match rx.await {
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
        self.cmd_tx.send(cmd).await;
    }

    async fn receive_commands(cmd_tx: mpsc::Sender<Command>, mut cmd_rx: mpsc::Receiver<Command>) {
        let mut people: BTreeMap<Person, mpsc::Sender<Message>> = BTreeMap::new();
        loop {
            match cmd_rx.recv().await {
                Some(Command::Join { person, res }) => {
                    let msg = format!("* {} has entered the room\n", person.name);
                    people.values().for_each(|msg_tx| {
                        let tx_clone = msg_tx.clone();
                        let msg_clone = msg.clone();
                        tokio::spawn(async move {
                            tx_clone.send(msg_clone).await;
                        });
                    });
                    let (tx1, rx1) = mpsc::channel(1);
                    let (tx2, rx2) = mpsc::channel(1);
                    let ours = Endpoint { tx: tx1, rx: rx2 };
                    let theirs = Endpoint { tx: tx2, rx: rx1 };
                    let names = people
                        .keys()
                        .map(|person| person.name.clone())
                        .collect::<Vec<String>>()
                        .join(", ");
                    let msg = format!("* The room contains: {}\n", names);
                    if res.send((msg, theirs)).is_ok() {
                        people.insert(person.clone(), ours.tx);
                        tokio::spawn(Self::receive_messages(person, ours.rx, cmd_tx.clone()));
                    }
                }
                Some(Command::Say { person, message }) => people
                    .iter()
                    .filter(|(them, _)| **them != person)
                    .for_each(|(them, msg_tx)| {
                        let msg_tx_clone = msg_tx.clone();
                        eprintln!("say say {:?} from {:?}", message, person);
                        let msg = format!("[{}] {}", person.name, message);
                        tokio::spawn(async move {
                            msg_tx_clone.send(msg).await;
                        });
                    }),
                Some(Command::Leave { person }) => {
                    if people.remove(&person).is_some() {
                        let msg = format!("* {} has left the room\n", person.name);
                        // copied from 50
                        people.values().for_each(|msg_tx| {
                            let tx_clone = msg_tx.clone();
                            let msg_clone = msg.clone();
                            tokio::spawn(async move {
                                tx_clone.send(msg_clone).await;
                            });
                        });
                    }
                }
                None => {
                    break;
                }
            }
        }
    }

    async fn receive_messages(
        person: Person,
        mut msg_rx: mpsc::Receiver<Message>,
        cmd_tx: mpsc::Sender<Command>,
    ) {
        loop {
            match msg_rx.recv().await {
                Some(message) => {
                    if cmd_tx
                        .send(Command::Say {
                            person: person.clone(),
                            message,
                        })
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                None => {
                    cmd_tx
                        .send(Command::Leave {
                            person: person.clone(),
                        })
                        .await;
                    break;
                }
            }
        }
    }
}
