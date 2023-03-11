use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use async_trait::async_trait;
use axum::extract::ws::{Message, WebSocket as WebSocketConn};
use axum::extract::{ConnectInfo, WebSocketUpgrade};
use axum::response::{IntoResponse};
use axum::{headers, Router, TypedHeader};
use axum::routing::get;
use futures::StreamExt;
use headers_client_ip::XRealIP;
use hyper::StatusCode;
use lazy_static::lazy_static;
use log::{debug, info, warn};
use prometheus::{Gauge, register_gauge};
use rhiaqey_sdk::gateway::{Gateway, GatewayConfig, GatewayMessage, GatewayMessageReceiver};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};

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

#[derive(Deserialize)]
pub struct Params {
    //
}

fn user_ip_allowed(
    ip: Option<TypedHeader<XRealIP>>,
    allowed_ips: Vec<String>,
) -> bool {
    if cfg!(debug_assertions) {
        return true
    }

    let Some(ip) = ip else {
        warn!("user ip was not found in headers");
        return false;
    };

    let Some(found) = allowed_ips.into_iter().
        find(|allowed_ip| allowed_ip.eq(&ip.to_string())) else {
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
    ip: Option<TypedHeader<XRealIP>>,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    state: Arc<WebSocketState>,
) -> impl IntoResponse {
    info!("[GET] Handle websocket connection");

    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };

    debug!("`{}` at {} connected.", user_agent, addr.to_string());

    let statx = state.clone();
    let settings = statx.settings.lock().await;
    let whitelisted_ips = settings.whitelisted_ips.clone();

    if !user_ip_allowed(ip, whitelisted_ips) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized access".to_string()).into_response();
    }

    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| {
        handle_ws_connection(
            socket,
            addr,
            state
        )
    })
}

/// Actual websocket state machine (one will be spawned per connection)
async fn handle_ws_connection(
    socket: WebSocketConn,
    who: SocketAddr,
    state: Arc<WebSocketState>,
) {
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
                        Err(error) => warn!("error parsing gateway message {error}")
                    }
                }
                Message::Binary(raw) => {
                    match serde_json::from_slice::<GatewayMessage>(raw.as_slice()) {
                        Ok(gateway_message) => {
                            debug!("gateway message arrived (text)");
                            sender.send(gateway_message).expect("could not send gateway message upstream");
                        }
                        Err(error) => warn!("error parsing gateway message {error}")
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
        info!("setting up {}", self.kind());

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
        info!("starting {}", self.kind());

        let sender = self.sender.clone().unwrap();
        let settings = self.settings.clone();
        let config = self.config.clone();

        tokio::task::spawn(async move {
            let config = config.lock().await;
            let settings = Arc::clone(&settings);
            let host = config.host.clone().unwrap_or(String::from("0.0.0.0"));
            let sadr = SocketAddr::from((host.parse::<IpAddr>().unwrap(), config.port));

            debug!("running websocket on address {}", sadr);

            let shared_state = Arc::new(WebSocketState{
                sender: Some(sender),
                settings,
            });

            let app = Router::new().
                route("/ws", get({
                    let shared_state = Arc::clone(&shared_state);
                    move |ip, ws, user_agent, info| {
                        ws_handler(ws, ip, user_agent, info, shared_state)
                    }
                }));

            axum::Server::bind(&sadr)
                .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                .await
                .unwrap();
        });
    }

    fn kind(&self) -> String {
        "websocket".to_string()
    }
}
