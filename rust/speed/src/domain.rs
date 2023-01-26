use std::collections::{BTreeMap, BTreeSet};

// Messages are sent between clients and the server.
//
// The server expects a client to identify its type.
//
// server: {IAmCamera: [Plate]*, IAmDispatcher: []} with one WantHeartbeat allowed at any point
// camera clients expect only heartbeats, dispatchers also expect tickets.
//
// TODO how can we statically encode the code on the cases?
// TODO if we did, would that affect the dispatch speed in reads, writes, etc.?
// TODO how can we encode the direction, and/or the state sequence constraints on the message types
// TODO how can we construct de/serialization code for these declaratively?
pub enum Message {
    // server -> client
    Error {
        msg: String,
    },
    Plate(Plate, Timestamp),
    Ticket(Ticket),
    WantHeartbeat {
        interval: Deciseconds,
    },
    Heartbeat,
    IAmCamera(Camera),
    IAmDispatcher {
        // Note this is redundant with roads.len(), just a serialization artifact.
        // Do we keep it in the struct or compute it at serialization?
        numroads: u8,
        roads: Vec<Road>,
    },
}

pub type Timestamp = u32;

pub type Road = u16;

pub type Mile = u16;

pub type Speed = u16;

pub type Deciseconds = u32;

pub type Plate = String;

pub struct Camera {
    pub road: Road,
    pub mile: Mile,
    pub limit: Speed,
}

pub struct Dispatcher {
    roads: BTreeSet<Road>,
}

pub struct Ticket {
    pub plate: Plate,
    pub road: Road,
    pub mile1: Mile,
    pub timestamp1: Timestamp,
    pub mile2: Mile,
    pub timestamp2: Timestamp,
    pub speed: Speed,
}

struct TimeRanges {
    asc: BTreeMap<Timestamp, Timestamp>,
    desc: BTreeMap<Timestamp, Timestamp>,
}

impl TimeRanges {
    fn new(&self) -> Self {
        Self {
            asc: BTreeMap::new(),
            desc: BTreeMap::new(),
        }
    }

    fn insert(&mut self, lower: Timestamp, upper: Timestamp) {
        self.asc.insert(lower, upper);
        self.desc.insert(upper, lower);
    }

    // Returns true if the given range bounds inclusively intersect with any
    // of the ranges in the set.
    fn intersects_any(&self, lower: Timestamp, upper: Timestamp) -> bool {
        if let Some((their_upper, _)) = self.desc.range(0..lower).last() {
            if *their_upper >= lower {
                return true;
            }
        }
        if let Some((their_lower, _)) = self.asc.range(upper..).last() {
            if *their_lower <= upper {
                return true;
            }
        }
        false
    }
}

// we need to be able to:
// 1. record a plate for a camera at a time
// 2. check the records of the nearest cameras in both directions in space for their closest
// observations in both directions in time and to see if a violation has occurred, resolving the
// time, and the mile of the camera if so
pub struct Region {
    // The set of violations that resulted in tickets.
    tickets: BTreeSet<Ticket>,
    // The set of tickets that are waiting to be issued.
    to_issue: BTreeSet<Ticket>,
    // The set of times for which a plate may not be issued another ticket.
    unticketable_ranges: BTreeMap<Plate, TimeRanges>,
    // The set of plate records.
    records: BTreeMap<Plate, BTreeMap<Road, BTreeMap<Mile, BTreeSet<Timestamp>>>>,
}

impl Region {
    pub fn new(&self) -> Self {
        Self {
            tickets: BTreeSet::new(),
            to_issue: BTreeSet::new(),
            unticketable_ranges: BTreeMap::new(),
            records: BTreeMap::new(),
        }
    }

    // records an observation of a plate by a camera
    pub fn record_plate(camera: Camera, plate: Plate) {
        todo!()
    }

    // returns a ticket to dispatch, if there is any. Tickets are issued only once.
    pub fn issue_ticket() -> Option<(Ticket, Dispatcher)> {
        todo!()
    }

    fn do_record_plate(camera: Camera, plate: Plate) -> Option<Ticket> {
        todo!()
    }

    fn do_record_ticket(ticket: Ticket) {
        todo!()
    }
}
