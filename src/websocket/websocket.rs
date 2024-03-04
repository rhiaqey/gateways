use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use async_trait::async_trait;
use axum::extract::ws::{Message, WebSocket as WebSocketConn};
use axum::extract::WebSocketUpgrade;
use axum::response::IntoResponse;
use axum::Router;
use axum::extract::State;
use axum::routing::get;
use futures::StreamExt;
use lazy_static::lazy_static;
use log::{debug, info, warn};
use prometheus::{Gauge, register_gauge};
use rhiaqey_sdk_rs::gateway::{Gateway, GatewayConfig, GatewayMessage, GatewayMessageReceiver};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};
use axum_client_ip::InsecureClientIp;
use hyper::http::StatusCode;
use axum_extra::headers;
use axum_extra::TypedHeader;
use serde_json::{json, Value};
use rhiaqey_sdk_rs::settings::Settings;

lazy_static! {
    static ref TOTAL_CONNECTIONS: Gauge = register_gauge!(
        "total_connections",
        "Total number of active connections.",
    ).expect("cannot create gauge metric for channels");
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WebSocketSettings {
    #[serde(alias = "WhitelistedIPs")]
    pub whitelisted_ips: Vec<String>,
}

impl Default for WebSocketSettings {
    fn default() -> Self {
        WebSocketSettings {
            whitelisted_ips: vec!(),
        }
    }
}

impl Settings for WebSocketSettings {
    //
}

#[derive(Default, Debug)]
pub struct WebSocket {
    sender: Option<UnboundedSender<GatewayMessage>>,
    settings: Arc<Mutex<WebSocketSettings>>,
    config: Arc<Mutex<GatewayConfig>>,
}

#[derive(Debug)]
pub struct WebSocketState {
    sender: Option<UnboundedSender<GatewayMessage>>,
    settings: Arc<Mutex<WebSocketSettings>>,
}

async fn get_home() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

fn user_ip_allowed(
    ip: &str,
    allowed_ips: Vec<String>,
) -> bool {
    if cfg!(debug_assertions) {
        return true
    }

    let Some(found) = allowed_ips.into_iter().
        find(|allowed_ip| allowed_ip.eq(&ip)) else {
        warn!("user ip {:?} is not allowed", ip);
        return false;
    };

    debug!("user ip {:?} is allowed", found);

    true
}

/// The handler for the HTTP request (this gets called when the HTTP GET lands at the start
/// of websocket negotiation). After this completes, the actual switching from HTTP to
/// websocket protocol will occur.
/// This is the last point where we can extract TCP/IP metadata such as IP address of the client
/// as well as things from HTTP headers such as user-agent of the browser etc.
async fn ws_handler(
    ws: WebSocketUpgrade,
    // headers: HeaderMap,
    insecure_ip: InsecureClientIp,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    State(state): State<Arc<WebSocketState>>,
) -> impl IntoResponse {
    info!("[GET] Handle websocket connection");

    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };

    let ip = insecure_ip.0.to_string();
    debug!("`{}` at {} connected.", user_agent, ip);

    let statx = state.clone();
    let settings = statx.settings.lock().await;
    let whitelisted_ips = settings.whitelisted_ips.clone();

    if !user_ip_allowed(&ip, whitelisted_ips) {
        return (StatusCode::FORBIDDEN, "Unauthorized access").into_response();
    }

    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| {
        handle_ws_connection(
            socket,
            ip,
            state
        )
    })
}

/// Actual websocket state machine (one will be spawned per connection)
async fn handle_ws_connection(
    socket: WebSocketConn,
    who: String,
    state: Arc<WebSocketState>,
) {
    info!("handle websocket connection: {}", who);

    let sender = state.sender.clone().unwrap();
    let (_, mut receiver) = socket.split();

    tokio::task::spawn(async move {
        'outer: while let Some(Ok(msg)) = receiver.next().await {
            debug!("received message {:?}", msg);

            match msg {
                Message::Ping(_) => {
                    // handled automatically by axum
                }
                Message::Pong(_) => {
                    // handled automatically by axum
                }
                Message::Text(txt) => {
                    match serde_json::from_str::<GatewayMessage>(txt.as_str()) {
                        Ok(gateway_message) => {
                            debug!("gateway message arrived (text)");
                            sender.send(gateway_message).expect("could not send gateway message upstream");
                        }
                        Err(error) => {
                            warn!("error parsing gateway text message: {error}");
                        }
                    }
                }
                Message::Binary(raw) => {
                    match serde_json::from_slice::<GatewayMessage>(raw.as_slice()) {
                        Ok(gateway_message) => {
                            debug!("gateway message arrived (text)");
                            sender.send(gateway_message).expect("could not send gateway message upstream");
                        }
                        Err(error) => {
                            warn!("error parsing gateway binary message: {error}");
                        }
                    }
                }
                Message::Close(c) => {
                    if let Some(cf) = c {
                        warn!(
                            "{} sent close with code {} and reason `{}`",
                            who, cf.code, cf.reason
                        );
                    } else {
                        warn!("{} somehow sent close message without CloseFrame", who);
                    }

                    TOTAL_CONNECTIONS.dec();

                    break 'outer;
                }
            }
        }
    });

    TOTAL_CONNECTIONS.inc();
}

#[async_trait]
impl Gateway<WebSocketSettings> for WebSocket {

    fn setup(&mut self, config: GatewayConfig, settings: Option<WebSocketSettings>) -> GatewayMessageReceiver {
        info!("setting up {}", Self::kind());

        self.config = Arc::new(Mutex::new(config));

        self.settings = Arc::new(Mutex::new(
            settings.unwrap_or(WebSocketSettings::default()),
        ));

        let (sender, receiver) = unbounded_channel::<GatewayMessage>();
        self.sender = Some(sender);

        Ok(receiver)
    }

    async fn set_settings(&mut self, settings: WebSocketSettings) {
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

            let shared_state = Arc::new(WebSocketState{
                sender: Some(sender),
                settings,
            });

            let app = Router::new()
                .route("/", get(get_home))
                .route("/ws", get(ws_handler))
                .with_state(shared_state);

            let host = config.host.clone().unwrap_or(String::from("0.0.0.0"));
            let addr = SocketAddr::from((host.parse::<IpAddr>().unwrap(), config.port));
            let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

            axum::serve(
                listener,
                // app.into_make_service()
                app.into_make_service_with_connect_info::<SocketAddr>(),
            ).await.unwrap();
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
        "websocket".to_string()
    }
}
