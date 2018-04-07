use super::event::EventInfo;
use super::stats::StatsRequest;

pub enum Control {
    Event(EventInfo),
    Stats(StatsRequest)
}
