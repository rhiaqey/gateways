use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use log::{debug, info};
use rhiaqey_sdk_rs::gateway::{Gateway, GatewayConfig, GatewayMessage, GatewayMessageReceiver};
use rhiaqey_sdk_rs::settings::Settings;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::Mutex;

#[derive(Default, Debug)]
pub struct HTTP {
    sender: Option<UnboundedSender<GatewayMessage>>,
    settings: Arc<Mutex<HTTPSettings>>,
    config: Arc<Mutex<GatewayConfig>>,
}

async fn get_home() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

impl Gateway<HTTPSettings> for HTTP {
    fn setup(
        &mut self,
        config: GatewayConfig,
        settings: Option<HTTPSettings>,
    ) -> GatewayMessageReceiver {
        info!("setting up {}", Self::kind());

        self.config = Arc::new(Mutex::new(config));

        self.settings = Arc::new(Mutex::new(settings.unwrap_or(HTTPSettings::default())));

        let (sender, receiver) = unbounded_channel::<GatewayMessage>();
        self.sender = Some(sender);

        Ok(receiver)
    }

    async fn set_settings(&mut self, settings: HTTPSettings) {
        let mut locked_settings = self.settings.lock().await;
        *locked_settings = settings;
        debug!("new settings updated");
    }

    async fn start(&mut self) {
        info!("starting {}", Self::kind());

        let sender = self.sender.clone().unwrap();
        let settings = self.settings.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let config = config.lock().await;
            let settings = Arc::clone(&settings);

            let app = Router::new()
                .route("/", get(get_home))
                // .with_state(shared_state);
                ;

            let host = config.host.clone().unwrap_or(String::from("0.0.0.0"));
            let addr = SocketAddr::from((host.parse::<IpAddr>().unwrap(), config.port));
            let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

            debug!("running at http://{host}:{}", config.port);

            axum::serve(
                listener,
                // app.into_make_service()
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
        });
    }

    fn schema() -> Value {
        json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "WhitelistedIPs": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "format": "ipv4",
                        "examples": [ "192.168.0.1", "10.0.0.1" ]
                    }
                }
            },
            "required": [ "WhitelistedIPs" ],
            "additionalProperties": false
        })
    }

    async fn metrics(&self) -> Value {
        json!({})
    }

    fn kind() -> String {
        String::from("http")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPSettings {
    //
}

impl Default for HTTPSettings {
    fn default() -> Self {
        HTTPSettings {}
    }
}

impl Settings for HTTPSettings {
    //
}
