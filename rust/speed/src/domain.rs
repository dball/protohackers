use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    time::Duration,
};

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
    Error(String),
    // camera -> server
    Plate(Plate, Timestamp),
    // server -> dispatcher
    Ticket(Ticket),
    // client -> server
    WantHeartbeat(Duration),
    // server -> client
    Heartbeat,
    // (client->camera) -> server
    IAmCamera(Camera),
    // (client->dispatcher) -> server
    IAmDispatcher(Dispatcher),
}

pub type Timestamp = u32;

pub type Road = u16;

pub type Mile = u16;

pub type Speed = u16;

pub type Plate = String;

#[derive(Clone, Copy)]
pub struct Camera {
    pub road: Road,
    pub mile: Mile,
    pub limit: Speed,
}

pub struct Dispatcher {
    pub roads: BTreeSet<Road>,
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

pub struct Region {
    // The set of tickets that have yet be issued.
    tickets_to_issue: BTreeMap<Road, VecDeque<Ticket>>,
    // The set of days for which a plate has already been issued a ticket.
    tickets_issued_days: BTreeMap<Plate, BTreeSet<Timestamp>>,
    // The set of plate records, indexed by plate, road, timestamp, then mile.
    records: BTreeMap<Plate, BTreeMap<Road, BTreeMap<Timestamp, Mile>>>,
}

impl Region {
    pub fn new() -> Self {
        Self {
            tickets_to_issue: BTreeMap::new(),
            tickets_issued_days: BTreeMap::new(),
            records: BTreeMap::new(),
        }
    }

    // records an observation of a plate by a camera. This will also record a ticket to issue if
    // the observation indicates a speed violation and the plate has not already been issued a ticket
    // for any of the ticket days.
    pub fn record_plate(&mut self, camera: Camera, plate: Plate, timestamp: Timestamp) {
        if let Some(ticket) = self.do_record_plate(camera, plate.clone(), timestamp) {
            let day1 = ticket.timestamp1 / 86400;
            let day2 = ticket.timestamp2 / 86400;
            let tickets_issued_days = self.tickets_issued_days.entry(plate).or_default();
            for day in day1..day2 + 1 {
                if tickets_issued_days.contains(&day) {
                    return;
                }
            }
            let tickets_for_road = self.tickets_to_issue.entry(ticket.road).or_default();
            tickets_for_road.push_back(ticket);
            for day in day1..day2 + 1 {
                if tickets_issued_days.insert(day) {
                    return;
                }
            }
        }
    }

    // returns a ticket to dispatch, if there are any for the given dispatcher. Tickets are issued only once.
    pub fn issue_ticket(&mut self, dispatcher: Dispatcher) -> Option<Ticket> {
        for road in dispatcher.roads.iter() {
            if let Some(tickets) = self.tickets_to_issue.get_mut(&road) {
                if let Some(ticket) = tickets.pop_front() {
                    return Some(ticket);
                }
            }
        }
        None
    }

    // records a plate observation and returns a ticket if it indicated a speed violation.
    fn do_record_plate(
        &mut self,
        camera: Camera,
        plate: Plate,
        timestamp: Timestamp,
    ) -> Option<Ticket> {
        let by_road = self.records.entry(plate.clone()).or_default();
        let by_timestamp = by_road.entry(camera.road).or_default();
        if by_timestamp.insert(timestamp, camera.mile).is_none() {
            if let Some((then, there)) = by_timestamp.range(0..timestamp).last() {
                if let Some(ticket) = Self::compute_ticket(camera, plate, timestamp, *then, *there)
                {
                    return Some(ticket);
                }
            } else if let Some((then, there)) = by_timestamp.range(timestamp + 1..).next() {
                if let Some(ticket) = Self::compute_ticket(camera, plate, timestamp, *then, *there)
                {
                    return Some(ticket);
                }
            }
        }
        None
    }

    // returns a ticket if the observations indicate a speed violation.
    fn compute_ticket(
        camera: Camera,
        plate: Plate,
        now: Timestamp,
        then: Timestamp,
        there: Mile,
    ) -> Option<Ticket> {
        let distance = camera.mile - there;
        let elapsed = now - then;
        let fspeed: f64 = f64::from(distance) / f64::from(elapsed);
        if fspeed.abs() > camera.limit.into() {
            let (mile1, mile2, timestamp1, timestamp2) = if fspeed > 0.0 {
                (camera.mile, there, now, then)
            } else {
                (there, camera.mile, then, now)
            };
            return Some(Ticket {
                plate,
                road: camera.road,
                mile1,
                timestamp1,
                mile2,
                timestamp2,
                speed: (fspeed.abs() * 100.0).round() as u16,
            });
        }
        None
    }
}
