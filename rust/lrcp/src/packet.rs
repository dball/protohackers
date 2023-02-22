use regex::bytes::{Captures, Regex};
use std::cmp::{max, min};
use std::collections::VecDeque;
use std::fmt::Debug;
use std::io::Read;
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
    ranges: VecDeque<Range<Position>>,
    range_cursor: Position,
}

// TODO if we didn't want to hold onto source for the entire read, we could either
// nibble off its front ranges as we go. Alternately, if we want to leave source
// alone, maybe we should box up an array instead, if that's a thing you can do.
impl Read for Data {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.len() == 0 {
            return Ok(0);
        }
        let mut buf_offset = 0;
        loop {
            let range = self.ranges.front();
            if range.is_none() {
                break;
            }
            let range = range.unwrap();
            let possible = range.len();
            let wanted = buf.len() - buf_offset;
            let taking = min(possible, wanted);
            let source = &self.source[range.start..range.start + taking];
            buf[buf_offset..buf_offset + taking].copy_from_slice(source);
            buf_offset += taking;
            if taking == range.len() {
                self.ranges.pop_front();
            }
        }
        Ok(buf_offset)
    }
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
        let mut ranges: VecDeque<Range<Position>> = VecDeque::with_capacity(1);
        {
            let mut offset = 0;
            // TODO could we use regex split instead of doing this manually and still get ranges?
            // TODO alternately, should we like box up the slices instead of using ranges?
            SLASHES
                .find_iter(body)
                .map(|m| m.start())
                .for_each(|m_start| {
                    if !(m_start == 0 && offset == 0) {
                        ranges.push_back(start + offset..start + m_start);
                    }
                    offset = m_start + 1;
                });
            if let Some(range) = ranges.back() {
                if range.end < stop {
                    ranges.push_back(range.end + 1..stop);
                }
            } else {
                ranges.push_back(start..stop);
            }
        }
        let data = Data {
            session,
            position,
            source: data,
            ranges,
            range_cursor: 0,
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
            ranges: VecDeque::from([15..21]),
            range_cursor: 0,
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
            ranges: VecDeque::from([15..18, 19..23, 24..28]),
            range_cursor: 0,
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
            ranges: VecDeque::from([16..17, 18..19, 20..21]),
            range_cursor: 0,
        };
        assert_eq!(Message::Data(data), parse_message(message).unwrap());
    }

    #[test]
    fn test_read_message_data() {
        let message = b"/data/12345/23/foo\\/bar\\\\baz/".to_vec();
        let mut data = Data {
            session: 12345,
            position: 23,
            source: message.clone(),
            ranges: VecDeque::from([15..18, 19..23, 24..28]),
            range_cursor: 0,
        };
        let mut buf: Vec<u8> = vec![];
        let n = data.read_to_end(&mut buf).unwrap();
        assert_eq!(n, 11);
        assert_eq!(b"foo/bar\\baz".to_vec(), buf);
    }
}
