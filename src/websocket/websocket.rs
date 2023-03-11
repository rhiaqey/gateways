use std::sync::Arc;
use async_trait::async_trait;
use rhiaqey_sdk::gateway::{Gateway, GatewayMessage, GatewayMessageReceiver};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};

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
}

#[async_trait]
impl Gateway<WebSocketSettings> for WebSocket {
    fn setup(&mut self, settings: Option<WebSocketSettings>) -> GatewayMessageReceiver {
        todo!()
    }
    async fn set_settings(&mut self, settings: WebSocketSettings) {
        todo!()
    }
    async fn start(&mut self) {
        todo!()
    }
    fn kind(&self) -> String {
        todo!()
    }
}
