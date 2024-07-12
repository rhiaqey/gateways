use prometheus::{labels, opts, register_gauge, Gauge};
use rhiaqey_common::env::Env;
use tokio::sync::OnceCell;

pub static TOTAL_CHANNELS: OnceCell<Gauge> = OnceCell::const_new();

pub static UP_INDICATOR: OnceCell<Gauge> = OnceCell::const_new();

pub async fn init_metrics(env: &Env, kind: String) {
    let id = env.get_id();
    let name = env.get_name();
    let namespace = env.get_namespace();
    let organization = env.get_organization();

    let values = labels! {
        "name" => name.as_str(),
        "id" => id.as_str(),
        "kind" => kind.as_str(),
        "namespace" => namespace.as_str(),
        "org" => organization
    };

    TOTAL_CHANNELS
        .get_or_init(|| async {
            register_gauge!(opts!(
                "rq_total_channels",
                "Total number of gateway channels.",
                values
            ))
            .unwrap()
        })
        .await;

    UP_INDICATOR
        .get_or_init(|| async {
            register_gauge!(opts!(
                "rq_up",
                "Whether the application is up (1) or down (0)",
                values
            ))
            .unwrap()
        })
        .await;
}
