use std::collections::BTreeMap;
use futures::sync::mpsc::Sender;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Stats {
    pub applications: BTreeMap<String, AppStats>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppStats {
    pub start_time: i64,
    pub running_time: i64
}

pub struct StatsRequest {
    pub feedback: Sender<Stats>
}
