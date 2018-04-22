use super::event::EventInfo;
use super::stats::StatsRequest;
use super::app::AppMigration;
use futures::sync::mpsc::Sender;

#[allow(dead_code)]
pub enum Control {
    Event(EventInfo),
    Stats(StatsRequest),
    ActivateMigration { app_id: usize, migration: AppMigration },
    MigrateAway { app_id: usize, sender: Sender<AppMigration> }
}
