use std::collections::{BTreeMap, BTreeSet};

use tokio::sync::mpsc;

pub type Timestamp = u32;
pub type Road = u16;
pub type Mile = u16;
pub type Speed = u16;
pub type Plate = String;

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub road: Road,
    pub mile: Mile,
    pub limit: Speed,
}

#[derive(Clone, Debug, Default)]
pub struct Dispatcher {
    pub roads: BTreeSet<Road>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ticket {
    pub plate: Plate,
    pub road: Road,
    pub mile1: Mile,
    pub timestamp1: Timestamp,
    pub mile2: Mile,
    pub timestamp2: Timestamp,
    pub speed: Speed,
}

#[derive(Clone, Debug)]
pub struct Observation {
    pub camera: Camera,
    pub plate: Plate,
    pub time: Timestamp,
}

pub struct Region {
    observations_tx: mpsc::UnboundedSender<Observation>,
    tickets_rx: mpsc::UnboundedReceiver<Ticket>,
}

impl Region {
    pub fn new() -> Self {
        let (observations_tx, observations_rx) = mpsc::unbounded_channel();
        let (violations_tx, violations_rx) = mpsc::channel(1);
        let (tickets_tx, tickets_rx) = mpsc::unbounded_channel();
        tokio::spawn(Self::do_record_observations(observations_rx, violations_tx));
        tokio::spawn(Self::do_assess_violations(violations_rx, tickets_tx));
        Self {
            observations_tx,
            tickets_rx,
        }
    }

    pub fn record_plate(&mut self, camera: Camera, plate: Plate, time: Timestamp) {
        self.observations_tx
            .send(Observation {
                camera,
                plate,
                time,
            })
            .expect("send to unbounded plates_seen");
    }

    async fn do_record_observations(
        mut observations_rx: mpsc::UnboundedReceiver<Observation>,
        violations_tx: mpsc::Sender<Ticket>,
    ) {
        let mut records = BTreeMap::new();
        loop {
            let obs = observations_rx.recv().await;
            if obs.is_none() {
                break;
            }
            if let Some(ticket) = Self::record_observation(&mut records, &obs.unwrap()) {
                violations_tx.send(ticket).await;
            }
        }
    }

    fn record_observation(
        records: &mut BTreeMap<Plate, BTreeMap<Road, BTreeMap<Timestamp, Mile>>>,
        obs: &Observation,
    ) -> Option<Ticket> {
        let by_road = records.entry(obs.plate.clone()).or_default();
        let by_timestamp = by_road.entry(obs.camera.road).or_default();
        if by_timestamp.insert(obs.time, obs.camera.mile).is_none() {
            if let Some((then, there)) = by_timestamp.range(0..obs.time).last() {
                Self::compute_ticket(obs.camera, obs.plate, obs.time, *then, *there)
            } else if let Some((then, there)) = by_timestamp.range(obs.time + 1..).next() {
                Self::compute_ticket(obs.camera, obs.plate, obs.time, *then, *there)
            } else {
                None
            }
        } else {
            None
        }
    }

    // returns a ticket if the observations indicate a speed violation.
    fn compute_ticket(
        camera: Camera,
        plate: Plate,
        now: Timestamp,
        then: Timestamp,
        there: Mile,
    ) -> Option<Ticket> {
        let here = camera.mile;
        let miles = f64::from(here) - f64::from(there);
        let hours = (f64::from(now) - f64::from(then)) / 3600.0;
        let velocity: f64 = miles / hours;
        let speed = velocity.abs();
        if speed > camera.limit.into() {
            let (mile1, mile2, timestamp1, timestamp2) = if then < now {
                (there, camera.mile, then, now)
            } else {
                (camera.mile, there, now, then)
            };
            return Some(Ticket {
                plate,
                road: camera.road,
                mile1,
                timestamp1,
                mile2,
                timestamp2,
                speed: (speed * 100.0).round() as u16,
            });
        }
        None
    }

    async fn do_assess_violations(
        mut violations_rx: mpsc::Receiver<Ticket>,
        tickets_tx: mpsc::UnboundedSender<Ticket>,
    ) {
        let mut tickets_issued: BTreeMap<Plate, BTreeSet<Timestamp>> = BTreeMap::new();
        'outer: loop {
            let ticket = violations_rx.recv().await;
            if ticket.is_none() {
                break;
            }
            let ticket = ticket.unwrap();
            let day1 = ticket.timestamp1 / 86400;
            let day2 = ticket.timestamp2 / 86400;
            let tickets_issued_days = tickets_issued.entry(ticket.plate.clone()).or_default();
            for day in day1..=day2 {
                if tickets_issued_days.contains(&day) {
                    break 'outer;
                }
            }
            if tickets_tx.send(ticket).is_err() {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::Region;

    use super::{Camera, Mile, Plate, Ticket, Timestamp};

    #[test]
    fn compute_ticket() {
        let camera = Camera {
            road: 0,
            mile: 1,
            limit: 10,
        };
        let plate: Plate = "A1".into();
        let now: Timestamp = 100;
        let then: Timestamp = 110;
        let there: Mile = 2;
        // 1 mile in 10 seconds is 6 miles a minute is 360 miles per hour
        let ticket = Ticket {
            plate: plate.clone(),
            road: 0,
            mile1: 1,
            timestamp1: 100,
            mile2: 2,
            timestamp2: 110,
            speed: 36000,
        };
        assert_eq!(
            Some(ticket),
            Region::compute_ticket(camera, plate, now, then, there)
        );
    }
}
