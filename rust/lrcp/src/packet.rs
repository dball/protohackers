use regex::bytes::{Captures, Regex};
use std::fmt::Debug;
use std::ops::Range;
use std::str::FromStr;

// Protohackers doesn't specify a max session value, actually, but I
// guess we'll start here for convenience.
pub type Session = u64;

// Numeric field values must be smaller than 2147483648.
// TODO this should be u32, not usize, but converting the ranges is confusing
// and distracting from more important lessons.
pub type Position = usize;

#[derive(Debug, PartialEq, Eq)]
pub struct Data {
    session: Session,
    position: Position,
    source: Vec<u8>,
    ranges: Vec<Range<Position>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Message {
    Connect(Session),
    Data(Data),
    Ack(Session, Position),
    Close(Session),
}

lazy_static! {
    static ref CONNECT: Regex = Regex::new(r"/connect/(?P<session>\d+)/").unwrap();
    static ref CLOSE: Regex = Regex::new(r"/close/(?P<session>\d+)/").unwrap();
    static ref DATA: Regex =
        Regex::new(r"/data/(?P<session>\d+)/(?P<position>\d+)/(?P<data>.*)/").unwrap();
    static ref ACK: Regex = Regex::new(r"/ack/(?P<session>\d+)/(?P<position>\d+)").unwrap();
    static ref SLASHES: Regex = Regex::new(r"\\(?:\\|/)").unwrap();
}

fn must_pluck<T: FromStr>(caps: &Captures, name: &str) -> T
where
    <T as FromStr>::Err: Debug,
{
    let value = std::str::from_utf8(caps.name(name).unwrap().as_bytes()).unwrap();
    value.parse::<T>().unwrap()
}

// TODO couldn't we use TryFrom idiomatically?
pub fn parse_message(data: Vec<u8>) -> anyhow::Result<Message> {
    if let Some(caps) = CONNECT.captures(&data) {
        let session: Session = must_pluck(&caps, "session");
        Ok(Message::Connect(session))
    } else if let Some(caps) = DATA.captures(&data) {
        let session: Session = must_pluck(&caps, "session");
        let position: Position = must_pluck(&caps, "position");
        let cap = caps.name("data").unwrap();
        let start = cap.start();
        let stop = cap.end();
        let body = &data.as_slice()[start..stop];
        let mut ranges: Vec<Range<Position>> = Vec::with_capacity(1);
        {
            let mut offset = 0;
            // TODO could we use regex split instead of doing this manually and still get ranges?
            // TODO alternately, should we like box up the slices instead of using ranges?
            SLASHES
                .find_iter(body)
                .map(|m| m.start())
                .for_each(|m_start| {
                    if !(m_start == 0 && offset == 0) {
                        ranges.push(start + offset..start + m_start);
                    }
                    offset = m_start + 1;
                });
            if let Some(range) = ranges.last() {
                if range.end < stop {
                    ranges.push(range.end + 1..stop);
                }
            } else {
                ranges.push(start..stop);
            }
        }
        let data = Data {
            session,
            position,
            source: data,
            ranges,
        };
        Ok(Message::Data(data))
    } else if let Some(caps) = ACK.captures(&data) {
        let session: Session = must_pluck(&caps, "session");
        let position: Position = must_pluck(&caps, "position");
        Ok(Message::Ack(session, position))
    } else if let Some(caps) = CLOSE.captures(&data) {
        let session: Session = must_pluck(&caps, "session");
        Ok(Message::Close(session))
    } else {
        Err(anyhow::anyhow!("invalid message"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_message_connect() {
        let message = b"/connect/12345/".to_vec();
        assert_eq!(Message::Connect(12345), parse_message(message).unwrap(),);
    }

    #[test]
    fn test_parse_message_data() {
        let message = b"/data/12345/23/foobar/".to_vec();
        let data = Data {
            session: 12345,
            position: 23,
            source: message.clone(),
            ranges: vec![15..21],
        };
        assert_eq!(Message::Data(data), parse_message(message).unwrap());
    }

    #[test]
    fn test_parse_message_data_with_slashes() {
        let message = b"/data/12345/23/foo\\/bar\\\\baz/".to_vec();
        let data = Data {
            session: 12345,
            position: 23,
            source: message.clone(),
            ranges: vec![15..18, 19..23, 24..28],
        };
        assert_eq!(Message::Data(data), parse_message(message).unwrap());
    }

    #[test]
    fn test_parse_message_data_with_all_slashes() {
        let message = b"/data/12345/23/\\/\\/\\//".to_vec();
        let data = Data {
            session: 12345,
            position: 23,
            source: message.clone(),
            ranges: vec![16..17, 18..19, 20..21],
        };
        assert_eq!(Message::Data(data), parse_message(message).unwrap());
    }
}
