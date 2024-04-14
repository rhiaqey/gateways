use rhiaqey_gateway_http::http::{HTTPSettings, HTTP};

#[tokio::main]
async fn main() {
    rhiaqey_gateways::exe::run::<HTTP, HTTPSettings>().await
}
