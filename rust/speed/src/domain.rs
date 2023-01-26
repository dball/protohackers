use std::collections::BTreeSet;

// Messages are sent between clients and the server.
//
// The server expects a client to identify its type.
//
// server: {IAmCamera: [Plate]*, IAmDispatcher: []} with one WantHeartbeat allowed at any point
// camera clients expect only heartbeats, dispatchers also expect tickets.
//
// TODO how can we statically encode the code on the struct cases?
// TODO if we did, would that affect the dispatch speed in reads, writes, etc.?
// TODO how can we encode the direction, and/or the state sequence constraints on the message types
// TODO how can we construct de/serialization code for these declaratively?
pub enum Message {
    // server -> client
    Error {
        msg: String,
    },
    Plate {
        plate: String,
        timestamp: Timestamp,
    },
    Ticket {
        plate: String,
        road: Road,
        mile1: Mile,
        timestamp1: Timestamp,
        mile2: Mile,
        timestamp2: Timestamp,
        speed: Speed,
    },
    WantHeartbeat {
        interval: Deciseconds,
    },
    Heartbeat,
    IAmCamera {
        road: Road,
        mile: Mile,
        limit: Speed,
    },
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
    road: Road,
    mile: Mile,
    limit: Speed,
}

pub struct Dispatcher {
    roads: BTreeSet<Road>,
}

pub struct Ticket {
    plate: Plate,
    road: Road,
    mile1: Mile,
    timestamp1: Timestamp,
    mile2: Mile,
    timestamp2: Timestamp,
    speed: Speed,
}

pub struct Region {
    // cameras, dispatchers, tickets
}

impl Region {
    // records an observation of a plate by a camera
    pub fn record_plate(camera: Camera, plate: Plate) {}

    // returns a ticket to dispatch, if there is any. Tickets are issued only once.
    pub fn issue_ticket() -> Option<(Ticket, Dispatcher)> {
        None
    }
}
