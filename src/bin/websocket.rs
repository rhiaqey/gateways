use rhiaqey_gateways::websocket::websocket::{WebSocket, WebSocketSettings};

#[tokio::main]
async fn main() {
    rhiaqey_gateways::exe::run::<WebSocket, WebSocketSettings>().await
}
