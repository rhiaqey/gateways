use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_client_ip::InsecureClientIp;
use axum_extra::{headers, TypedHeader};
use log::{debug, info, warn};
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

#[derive(Debug)]
struct HTTPState {
    sender: Option<UnboundedSender<GatewayMessage>>,
    settings: Arc<Mutex<HTTPSettings>>,
}
async fn get_home() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

fn user_ip_allowed(ip: &str, allowed_ips: Vec<String>) -> bool {
    if cfg!(debug_assertions) {
        return true;
    }

    let Some(found) = allowed_ips
        .into_iter()
        .find(|allowed_ip| allowed_ip.eq(&ip))
    else {
        warn!("user ip {:?} is not allowed", ip);
        return false;
    };

    debug!("user ip {:?} is allowed", found);

    true
}

async fn http_handler(
    // headers: HeaderMap,
    insecure_ip: InsecureClientIp,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    State(state): State<Arc<HTTPState>>,
    Json(payload): Json<GatewayMessage>,
) -> impl IntoResponse {
    info!("[GET] Handle http connection");

    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };

    let ip = insecure_ip.0.to_string();
    debug!("`{}` at {} connected.", user_agent, ip);

    let statx = state.clone();
    let sx = state.sender.clone().unwrap();
    let settings = statx.settings.lock().await;
    let whitelisted_ips = settings.whitelisted_ips.clone();

    if !user_ip_allowed(&ip, whitelisted_ips) {
        return (StatusCode::FORBIDDEN, "Unauthorized access").into_response();
    }

    sx.send(payload)
        .expect("could not send gateway message upstream");

    (StatusCode::ACCEPTED, "").into_response()
}

impl Gateway<HTTPSettings> for HTTP {
    async fn setup(
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

            let shared_state = Arc::new(HTTPState {
                sender: Some(sender),
                settings,
            });

            let app = Router::new()
                .route("/", get(get_home))
                .route("/http", post(http_handler))
                .route("/rest", post(http_handler))
                .with_state(shared_state);

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

    fn kind() -> String {
        String::from("http")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HTTPSettings {
    #[serde(alias = "WhitelistedIPs")]
    pub whitelisted_ips: Vec<String>,
}

impl Default for HTTPSettings {
    fn default() -> Self {
        HTTPSettings {
            whitelisted_ips: vec![],
        }
    }
}

impl Settings for HTTPSettings {
    //
}
