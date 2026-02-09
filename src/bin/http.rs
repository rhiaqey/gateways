use rhiaqey_gateway_http::http::{HTTP, HTTPSettings};

#[tokio::main]
async fn main() {
    rhiaqey_gateways::exe::run::<HTTP, HTTPSettings>().await
}
