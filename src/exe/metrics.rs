use lazy_static::lazy_static;
use prometheus::{IntGauge, register_int_gauge};

lazy_static! {
    pub(crate) static ref TOTAL_CHANNELS: IntGauge =
        register_int_gauge!("total_channels", "Total number of producer channels.").unwrap();
}
