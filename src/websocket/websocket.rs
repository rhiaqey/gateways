use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use async_trait::async_trait;
use axum::extract::ws::{Message, WebSocket as WebSocketConn};
use axum::extract::{ConnectInfo, Query, State, WebSocketUpgrade};
use axum::response::{IntoResponse, Response};
use axum::{Extension, headers, Router, TypedHeader};
use axum::routing::get;
use headers_client_ip::XRealIP;
use lazy_static::lazy_static;
use log::{debug, info};
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
    shared_state: Arc<Mutex<WebSocketState>>,
) -> impl IntoResponse {
    info!("[GET] Handle websocket connection");

    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };

    debug!("`{}` at {} connected.", user_agent, addr.to_string());

    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| {
        handle_ws_connection(
            socket,
            addr,
            // state,
        )
    })
}

/// Actual websocket state machine (one will be spawned per connection)
async fn handle_ws_connection(
    socket: WebSocketConn,
    who: SocketAddr,
    // state: Arc<WebSocketState>,
) {
    todo!();
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
            let setx = Arc::clone(&settings);
            let settings = settings.lock().await;
            let host = config.host.clone().unwrap_or(String::from("0.0.0.0"));
            let sadr = SocketAddr::from((host.parse::<IpAddr>().unwrap(), config.port));

            debug!("running websocket on address {}", sadr);

            let shared_state = Arc::new(Mutex::new(WebSocketState{
                sender: Some(sender),
                settings: setx,
            }));

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
