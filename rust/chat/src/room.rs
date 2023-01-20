use lazy_static::lazy_static;
use regex::Regex;
use tokio::sync::broadcast;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::oneshot;

enum Command {
    Enter { res: oneshot::Sender<Pair> },
    Join { person: Person, res: oneshot::Sender<()> },
}

#[derive(Debug)]
pub struct Room {
    cmd_sender: Sender<Command>,
}

pub struct Pair {
    pub sender: Sender<Message>,
    pub receiver: Receiver<Message>,
}

impl Room {
    pub fn new() -> Self {
        let (cmd_sender, cmd_receiver) = channel::<Command>(1);
        let (broadcast_sender, broadcast_receiver) = broadcast::channel::<Message>(16);
        tokio::spawn(async move { broadcast_logger(broadcast_receiver).await });
        tokio::spawn(async move { room_manager(cmd_receiver, broadcast_sender).await });
        Self { cmd_sender }
    }

    pub async fn enter(&self) -> Option<Pair> {
        let (res_sender, res_receiver) = oneshot::channel::<Pair>();
        if self.cmd_sender.send(Command::Enter{ res: res_sender }).await.is_err() {
            return None
        }
        match res_receiver.await {
            Ok(pair) => { Some(pair) },
            Err(_) => { None },
        }
    }

}

async fn broadcast_logger(mut receiver: broadcast::Receiver<Message>) {
    loop {
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

async fn room_manager(mut cmd_receiver: Receiver<Command>, broadcast_sender: broadcast::Sender<Message>) {
    loop {
        match cmd_receiver.recv().await {
            Some(cmd) => match cmd {
                Command::Enter { res } => {
                    let (their_tx, our_rx) = channel::<Message>(1);
                    let (our_tx, their_rx) = channel::<Message>(1);
                    let ours = Pair {
                        sender: our_tx,
                        receiver: their_rx,
                    };
                    let theirs = Pair {
                        sender: their_tx,
                        receiver: our_rx,
                    };
                    if res.send(theirs).is_ok() {
                        let their_sender = broadcast_sender.clone();
                        tokio::spawn(async move { person_manager(ours, their_sender) });
                    }
                },
                Command::Join { person, res } => {

                },
            },
            None => {
                break;
            }
        }
    }
}

async fn person_manager(mut ours: Pair, broadcast_sender: broadcast::Sender<Message>) {
    let msg = match ours.receiver.recv().await {
        Some(msg) => { msg }
        None => { return; }
    };
    let person = match Person::new(msg.contents) {
        Some(person) => { person },
        None => { return; }
    };
    let greeting = Message::new("joined".to_owned());
    let receipt = ours.sender.send(greeting).await;
    if receipt.is_err() {
        return;
    }
    loop {
        // select across:
        // 1. ours receiver -> send broadcast (or send message command to the room, not sure) or exit
        // 2. broadcast receiver -> send ours transmitter or exit
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub contents: String,
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
