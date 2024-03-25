use axum::routing::get;
use axum::Router;
use axum::{http::StatusCode, response::IntoResponse};
use log::{debug, info};
use prometheus::{Encoder, TextEncoder};
use std::net::SocketAddr;

async fn get_ready() -> impl IntoResponse {
    StatusCode::OK
}

async fn get_metrics() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    let mf = prometheus::gather();
    encoder.encode(&mf, &mut buffer).unwrap();
    (
        StatusCode::OK,
        [(
            hyper::header::CONTENT_TYPE,
            encoder.format_type().to_string(),
        )],
        buffer.into_response(),
    )
}

async fn get_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub async fn start_private_http_server(port: u16) {
    info!("starting http server @ port {}", port);

    let app = Router::new()
        .route("/alive", get(get_ready))
        .route("/ready", get(get_ready))
        .route("/metrics", get(get_metrics))
        .route("/version", get(get_version));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    debug!("running http server @ port {}", port);

    axum::serve(
        listener,
        // app.into_make_service()
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
