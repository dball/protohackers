use std::collections::{BTreeMap, BTreeSet, VecDeque};

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

#[derive(Clone, Debug)]
struct Dispatch {
    dispatcher: Dispatcher,
    tickets_tx: mpsc::Sender<Ticket>,
}

#[derive(Debug)]
pub struct Region {
    observations_tx: mpsc::UnboundedSender<Observation>,
    dispatches_tx: mpsc::UnboundedSender<Dispatch>,
}

impl Region {
    pub fn new() -> Self {
        let (observations_tx, observations_rx) = mpsc::unbounded_channel();
        let (violations_tx, violations_rx) = mpsc::channel(1);
        tokio::spawn(Self::do_record_observations(observations_rx, violations_tx));

        let (tickets_tx, tickets_rx) = mpsc::unbounded_channel();
        tokio::spawn(Self::do_assess_violations(violations_rx, tickets_tx));

        let (dispatches_tx, dispatches_rx) = mpsc::unbounded_channel();
        tokio::spawn(Self::do_manage_dispatchers(dispatches_rx, tickets_rx));

        Self {
            observations_tx,
            dispatches_tx,
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn record_plate(&mut self, camera: Camera, plate: Plate, time: Timestamp) {
        self.observations_tx
            .send(Observation {
                camera,
                plate,
                time,
            })
            .expect("send to unbounded observations");
    }

    #[tracing::instrument(skip_all)]
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
                tracing::info!(?ticket, "sending violation");
                if let Err(err) = violations_tx.send(ticket.clone()).await {
                    tracing::error!(?ticket, ?err, "error sending violation");
                }
            }
        }
    }

    #[tracing::instrument(skip(records))]
    fn record_observation(
        records: &mut BTreeMap<Plate, BTreeMap<Road, BTreeMap<Timestamp, Mile>>>,
        obs: &Observation,
    ) -> Option<Ticket> {
        let by_road = records.entry(obs.plate.clone()).or_default();
        let by_timestamp = by_road.entry(obs.camera.road).or_default();
        if by_timestamp.insert(obs.time, obs.camera.mile).is_none() {
            tracing::info!(?obs, "recorded observation");
            if let Some((then, there)) = by_timestamp.range(0..obs.time).last() {
                Self::compute_ticket(obs.camera, &obs.plate, obs.time, *then, *there)
            } else if let Some((then, there)) = by_timestamp.range(obs.time + 1..).next() {
                Self::compute_ticket(obs.camera, &obs.plate, obs.time, *then, *there)
            } else {
                None
            }
        } else {
            None
        }
    }

    // returns a ticket if the observations indicate a speed violation.
    #[tracing::instrument]
    fn compute_ticket(
        camera: Camera,
        plate: &Plate,
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
                plate: plate.clone(),
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

    #[tracing::instrument(skip_all)]
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
                    continue 'outer;
                }
            }
            tracing::info!(?ticket, "issuing ticket");
            if let Err(err) = tickets_tx.send(ticket) {
                tracing::error!(?err, "error issuing ticket");
                break;
            }
            for day in day1..=day2 {
                tickets_issued_days.insert(day);
            }
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn register_dispatcher(&mut self, dispatcher: Dispatcher) -> mpsc::Receiver<Ticket> {
        let (tickets_tx, tickets_rx) = mpsc::channel(1);
        let dispatch = Dispatch {
            dispatcher,
            tickets_tx,
        };
        self.dispatches_tx
            .send(dispatch)
            .expect("send to unbounded internal dispatches");
        tickets_rx
    }

    #[tracing::instrument(skip_all)]
    async fn do_manage_dispatchers(
        mut dispatches_rx: mpsc::UnboundedReceiver<Dispatch>,
        mut tickets_rx: mpsc::UnboundedReceiver<Ticket>,
    ) {
        let mut unsent: BTreeMap<Road, VecDeque<Ticket>> = BTreeMap::new();
        let mut dispatchers: BTreeMap<Road, VecDeque<mpsc::Sender<Ticket>>> = BTreeMap::new();
        'select: loop {
            tokio::select! {
                ticket = tickets_rx.recv() => {
                    if ticket.is_none() {
                        tracing::info!("ticket receiver channel closed, stopping");
                        break;
                    }
                    let ticket = ticket.unwrap();
                    let road_dispatchers = dispatchers.entry(ticket.road).or_default();
                    loop {
                        let dispatcher = road_dispatchers.front();
                        if dispatcher.is_none() {
                            break;
                        }
                        let dispatcher = dispatcher.unwrap();
                        tracing::info!(?ticket, ?dispatcher, "trying to dispatch ticket");
                        if let Err(err) = dispatcher.send(ticket.clone()).await {
                            tracing::warn!(?ticket, ?err, "failed to dispatch ticket");
                            road_dispatchers.pop_front();
                        } else {
                            tracing::info!(?ticket, ?dispatcher, "dispatched ticket");
                            continue 'select;
                        }
                    }
                    tracing::info!(?ticket, "recording ticket to send later");
                    unsent.entry(ticket.road).or_default().push_back(ticket);
                }
                dispatch = dispatches_rx.recv() => {
                    if dispatch.is_none() {
                        tracing::info!("dispatch receiver channel closed, stopping");
                        break;
                    }
                    let Dispatch { dispatcher, tickets_tx } = dispatch.unwrap();
                    for road in dispatcher.roads.iter() {
                        if let Some(tickets) = unsent.get_mut(&road) {
                            loop {
                                let ticket = tickets.pop_front();
                                if ticket.is_none() {
                                    break;
                                }
                                let ticket = ticket.unwrap();
                                if tickets_tx.send(ticket.clone()).await.is_err() {
                                    tickets.push_front(ticket);
                                    break 'select;
                                }
                            }
                        }
                    }
                    for road in dispatcher.roads.iter() {
                        let road_dispatchers = dispatchers.entry(*road).or_default();
                        road_dispatchers.push_back(tickets_tx.clone());
                    }
                }
            }
        }
        tracing::info!("stop");
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
            Region::compute_ticket(camera, &plate, now, then, there)
        );
    }
}
