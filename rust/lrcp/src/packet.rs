use regex::bytes::{Captures, Regex};
use std::fmt::Debug;
use std::str::FromStr;

// Protohackers doesn't specify a max session value, actually, but I
// guess we'll start here for convenience.
pub type Session = u64;

// Numeric field values must be smaller than 2147483648.
pub type Position = u32;

#[derive(Debug, Clone)]
pub enum Message<'a> {
    Connect(Session),
    Data(Session, Position, &'a [u8]),
    Ack(Session, Position),
    Close(Session),
}

lazy_static! {
    static ref CONNECT: Regex = Regex::new(r"/connect/(?P<session>\d+)/").unwrap();
    static ref CLOSE: Regex = Regex::new(r"/close/(?P<session>\d+)/").unwrap();
    static ref DATA: Regex =
        Regex::new(r"/data/(?P<session>\d+)/(?P<position>\d+)/(?P<data>.*)/").unwrap();
    static ref ACK: Regex = Regex::new(r"/ack/(?P<session>\d+)/(?P<position>\d+)").unwrap();
}

fn must_pluck<T: FromStr>(caps: &Captures, name: &str) -> T
where
    <T as FromStr>::Err: Debug,
{
    let value = std::str::from_utf8(caps.name(name).unwrap().as_bytes()).unwrap();
    value.parse::<T>().unwrap()
}

// TODO couldn't we use TryFrom idiomatically?
pub fn parse_message(data: &[u8]) -> anyhow::Result<Message> {
    if let Some(caps) = CONNECT.captures(data) {
        let session: Session = must_pluck(&caps, "session");
        Ok(Message::Connect(session))
    } else if let Some(caps) = DATA.captures(data) {
        let session: Session = must_pluck(&caps, "session");
        let position: Position = must_pluck(&caps, "position");
        let data = caps.name("data").unwrap().as_bytes();
        Ok(Message::Data(session, position, data))
    } else if let Some(caps) = ACK.captures(data) {
        let session: Session = must_pluck(&caps, "session");
        let position: Position = must_pluck(&caps, "position");
        Ok(Message::Ack(session, position))
    } else if let Some(caps) = CLOSE.captures(data) {
        let session: Session = must_pluck(&caps, "session");
        Ok(Message::Close(session))
    } else {
        Err(anyhow::anyhow!("invalid message"))
    }
}
