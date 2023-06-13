use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Bound::Excluded;
use std::ops::Bound::Unbounded;

use std::net::SocketAddr;
use tokio::sync::mpsc;

use crate::messages::ServerMessage::{self, Ticket};

type Road = u16;
type Mile = u16;
type Plate = String;
type Timestamp = u32;

#[derive(Clone, Copy)]
struct SortedSpeedAverage {
    pub mile1: Mile,
    pub timestamp1: Timestamp,
    pub mile2: Mile,
    pub timestamp2: Timestamp,
    pub average: u16,
}

//Function parameters do not have to be sorted, output is sorted
//Output speed average is in miles per hour
fn compute_average_speed(
    mile1: Mile,
    timestamp1: Timestamp,
    mile2: Mile,
    timestamp2: Timestamp,
) -> Option<SortedSpeedAverage> {
    if timestamp2 == timestamp1 {
        return None;
    }

    let mut mile1 = mile1;
    let mut mile2 = mile2;
    let mut timestamp1 = timestamp1;
    let mut timestamp2 = timestamp2;

    if timestamp2 < timestamp1 {
        (mile1, mile2) = (mile2, mile1);
        (timestamp1, timestamp2) = (timestamp2, timestamp1);
    }

    let average_speed = mile2.abs_diff(mile1) as u32 * (3600 / (timestamp2 - timestamp1));

    Some(SortedSpeedAverage {
        mile1,
        mile2,
        timestamp1,
        timestamp2,
        average: average_speed as u16,
    })
}

pub struct PlateObsv {
    pub plate: Plate,
    pub road: Road,
    pub mile: Mile,
    pub speed_limit: u16,
    pub timestamp: Timestamp,
}

pub enum DispatcherRegistration {
    Registration {
        addr: SocketAddr,
        roads: Vec<u16>,
        channel: mpsc::Sender<ServerMessage::Ticket>,
    },
    Deregister {
        addr: SocketAddr,
        roads: Vec<u16>,
    },
}

struct Dispatcher {
    addr: SocketAddr,
    channel: mpsc::Sender<ServerMessage::Ticket>,
}

pub struct Manager {
    plate_obsvervations: HashMap<Plate, HashMap<Road, BTreeMap<Mile, Timestamp>>>,
    plate_obsv_chan_rx: mpsc::Receiver<PlateObsv>,
    plate_obsv_chan_tx: mpsc::Sender<PlateObsv>,

    dipatcher_regis_chan_rx: mpsc::Receiver<DispatcherRegistration>,
    dipatcher_regis_chan_tx: mpsc::Sender<DispatcherRegistration>,

    day_ticket_sent: HashMap<Plate, HashSet<u32>>,
    dispatchers: HashMap<Road, Vec<Dispatcher>>,

    unsent_tickets: HashMap<Road, Vec<Ticket>>,
}

#[derive(Clone)]
pub struct PublicChannels {
    pub plate_obsv_chan_tx: mpsc::Sender<PlateObsv>,
    pub dispatcher_regis_chan_tx: mpsc::Sender<DispatcherRegistration>,
}

impl Manager {
    pub fn new() -> Manager {
        let plate_obsvervations = HashMap::new();
        let (plate_obsv_chan_tx, plate_obsv_chan_rx) = mpsc::channel(1024);

        let last_day_ticket_sent = HashMap::new();

        let (dipatcher_regis_chan_tx, dipatcher_regis_chan_rx) = mpsc::channel(512);
        let dispatchers = HashMap::new();

        let unsent_tickets = HashMap::new();

        Manager {
            plate_obsvervations,
            plate_obsv_chan_rx,
            plate_obsv_chan_tx,
            day_ticket_sent: last_day_ticket_sent,
            dispatchers,
            dipatcher_regis_chan_rx,
            dipatcher_regis_chan_tx,
            unsent_tickets,
        }
    }

    pub fn get_channels(&self) -> PublicChannels {
        PublicChannels {
            plate_obsv_chan_tx: self.plate_obsv_chan_tx.clone(),
            dispatcher_regis_chan_tx: self.dipatcher_regis_chan_tx.clone(),
        }
    }

    async fn dispatcher_registration(&mut self, reg: DispatcherRegistration) {
        match reg {
            DispatcherRegistration::Registration {
                addr,
                roads,
                channel,
            } => {
                for road in roads {
                    self.dispatchers
                        .entry(road)
                        .or_insert(Vec::new())
                        .push(Dispatcher {
                            addr,
                            channel: channel.clone(),
                        });
                    self.send_unsent_tickets(road).await;
                }
            }
            DispatcherRegistration::Deregister { addr, roads } => {
                for road in roads {
                    if let Some(vec) = self.dispatchers.get_mut(&road) {
                        let is_at = vec.iter_mut().position(|disp| disp.addr == addr);
                        if let Some(idx) = is_at {
                            vec.remove(idx);
                        }
                    }
                }
            }
        }
    }

    fn check_if_speeding(&self, plate_obsv: &PlateObsv) -> Option<SortedSpeedAverage> {
        let prev_seen_at_miles = self
            .plate_obsvervations
            .get(&plate_obsv.plate)
            .and_then(|roads| roads.get(&plate_obsv.road));

        if let Some(prev_seen_at_miles) = prev_seen_at_miles {
            let previous_mile_obsv = prev_seen_at_miles
                .range((Unbounded, Excluded(plate_obsv.mile)))
                .last();
            let next_mile_obsv = prev_seen_at_miles
                .range((Excluded(plate_obsv.mile), Unbounded))
                .next();

            let average_speed = previous_mile_obsv.and_then(|(&mile, &timestamp)| {
                compute_average_speed(plate_obsv.mile, plate_obsv.timestamp, mile, timestamp)
            });

            if average_speed.is_some_and(|avg| avg.average > plate_obsv.speed_limit) {
                return Some(average_speed.unwrap());
            }

            let average_speed = next_mile_obsv.and_then(|(&mile, &timestamp)| {
                compute_average_speed(plate_obsv.mile, plate_obsv.timestamp, mile, timestamp)
            });

            if average_speed.is_some_and(|avg| avg.average > plate_obsv.speed_limit) {
                return Some(average_speed.unwrap());
            }
        };

        None
    }

    async fn send_unsent_tickets(&mut self, road: u16) {
        while let Some(ticket) = self
            .unsent_tickets
            .get_mut(&road)
            .and_then(|tickets| tickets.pop())
        {
            if let Some(dispatcher) = self
                .dispatchers
                .get(&ticket.road)
                .and_then(|list| list.last())
            {
                if dispatcher.channel.send(ticket).await.is_err() {
                    println!("Err: Failed to send ticket to dispatcher.");
                }
            }
        }
    }

    async fn send_ticket(&mut self, plate_obsv: &PlateObsv, speeding_details: &SortedSpeedAverage) {
        let ticket = Ticket {
            plate: plate_obsv.plate.clone(),
            road: plate_obsv.road,
            mile1: speeding_details.mile1,
            timestamp1: speeding_details.timestamp1,
            mile2: speeding_details.mile2,
            timestamp2: speeding_details.timestamp2,
            speed: speeding_details.average * 100,
        };

        if let Some(dispatcher) = self
            .dispatchers
            .get(&ticket.road)
            .and_then(|list| list.last())
        {
            if dispatcher.channel.send(ticket).await.is_err() {
                println!("Err: Failed to send ticket to dispatcher.");
            }
        } else {
            self.unsent_tickets
                .entry(ticket.road)
                .or_insert(Vec::new())
                .push(ticket);
        }
    }

    async fn register_plate_observation(&mut self, plate_obsv: PlateObsv) {
        let curr_day = plate_obsv.timestamp / 86400;

        if !self
            .day_ticket_sent
            .entry(plate_obsv.plate.clone())
            .or_insert(HashSet::new())
            .contains(&curr_day)
        {
            if let Some(speeding_details) = self.check_if_speeding(&plate_obsv) {
                self.send_ticket(&plate_obsv, &speeding_details).await;
                self.day_ticket_sent
                    .get_mut(&plate_obsv.plate)
                    .unwrap()
                    .insert(curr_day);
            }
        }

        self.plate_obsvervations
            .entry(plate_obsv.plate)
            .or_insert(HashMap::new())
            .entry(plate_obsv.road)
            .or_insert(BTreeMap::new())
            .insert(plate_obsv.mile, plate_obsv.timestamp);
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                Some(plate_obsv) = self.plate_obsv_chan_rx.recv() => {
                    self.register_plate_observation(plate_obsv).await;
                }

                Some(reg) = self.dipatcher_regis_chan_rx.recv() => {
                    self.dispatcher_registration(reg).await;
                }
            }
        }
    }
}

impl Default for Manager {
    fn default() -> Self {
        Self::new()
    }
}
