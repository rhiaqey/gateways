use lazy_static::lazy_static;
use prometheus::{register_gauge, Gauge};

lazy_static! {
    pub static ref TOTAL_CHANNELS: Gauge = register_gauge!(
        "total_producer_channels",
        "Total number of active channels.",
    )
    .expect("cannot create gauge metric for channels");
}
