use rhiaqey_gateway_ws::websocket::{WebSocket, WebSocketSettings};

#[tokio::main]
async fn main() {
    rhiaqey_gateways::exe::run::<WebSocket, WebSocketSettings>().await
}
