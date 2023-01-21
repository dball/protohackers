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
        person_id: Id,
        message: Message,
    },
    Leave {
        person_id: Id,
    },
}

pub type Id = u64;
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

    async fn receive_commands(cmd_tx: mpsc::Sender<Command>, mut cmd_rx: mpsc::Receiver<Command>) {
        let mut next_id: Id = 1;
        let mut ids: BTreeMap<Person, Id> = BTreeMap::new();
        let mut endpoints: BTreeMap<Id, (Person, mpsc::Sender<Message>)> = BTreeMap::new();
        loop {
            match cmd_rx.recv().await {
                Some(Command::Join { person, res }) => {
                    let msg = format!("* {} has entered the room\n", person.name);
                    endpoints.values().for_each(|(_, msg_tx)| {
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
                    let names = ids
                        .keys()
                        .map(|person| person.name.clone())
                        .collect::<Vec<String>>()
                        .join(", ");
                    let msg = format!("* The room contains: {}\n", names);
                    if res.send((msg, theirs)).is_ok() {
                        let id = next_id;
                        next_id += 1;
                        ids.insert(person.clone(), id);
                        endpoints.insert(id, (person.clone(), ours.tx));
                        tokio::spawn(Self::receive_messages(id, ours.rx, cmd_tx.clone()));
                    }
                }
                Some(Command::Say { person_id, message }) => {}
                Some(Command::Leave { person_id }) => {}
                None => {
                    break;
                }
            }
        }
    }

    async fn receive_messages(
        id: Id,
        msg_rx: mpsc::Receiver<Message>,
        cmd_tx: mpsc::Sender<Command>,
    ) {
    }
}
