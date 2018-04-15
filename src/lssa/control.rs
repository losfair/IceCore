use super::event::EventInfo;
use super::stats::StatsRequest;

#[allow(dead_code)]
pub enum Control {
    Event(EventInfo),
    Stats(StatsRequest)
}
