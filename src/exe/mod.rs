mod http;
mod metrics;

use crate::exe::http::start_private_http_server;
use futures::StreamExt;
use log::{debug, info, trace, warn};
use rhiaqey_common::env::parse_env;
use rhiaqey_common::executor::{Executor, ExecutorPublishOptions};
use rhiaqey_common::pubsub::{PublisherRegistrationMessage, RPCMessage, RPCMessageData};
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use rhiaqey_sdk_rs::gateway::{Gateway, GatewayConfig};

use crate::exe::metrics::TOTAL_CHANNELS;

pub async fn run<
    P: Gateway<S> + Default + Send + 'static,
    S: DeserializeOwned + Default + Debug,
>() {
    env_logger::init();
    let env = parse_env();

    let mut executor = match Executor::setup(env).await {
        Ok(exec) => exec,
        Err(error) => {
            panic!("failed to setup executor: {error}");
        }
    };

    debug!(
        "gateway [id={},name={}] is ready",
        executor.get_id(),
        executor.get_name()
    );

    let mut plugin = P::default();
    let port = executor.get_private_port();
    let settings = executor.read_settings::<S>().await.unwrap_or(S::default());

    let config = GatewayConfig {
        port: executor.get_public_port(),
        host: None,
    };

    let mut publisher_stream = match plugin.setup(config, Some(settings)) {
        Err(error) => panic!("failed to setup gateway: {error}"),
        Ok(sender) => sender,
    };

    let publisher_registration_message = RPCMessage {
        data: RPCMessageData::RegisterPublisher(PublisherRegistrationMessage {
            id: executor.get_id(),
            name: executor.get_name(),
            namespace: executor.get_namespace(),
            schema: P::schema(),
        }),
    };

    executor
        .rpc(executor.get_namespace(), publisher_registration_message)
        .await
        .expect("Publisher must first register with hub");

    debug!("rpc registration message sent");

    plugin.start().await;

    tokio::spawn(start_private_http_server(port));

    let mut pubsub_stream = executor.create_hub_to_publishers_pubsub().await.unwrap();
    let channel_count = executor.get_channel_count().await as f64;
    TOTAL_CHANNELS.set(channel_count);

    debug!("stream is ready");

    loop {
        tokio::select! {
            Some(message) = publisher_stream.recv() => {
                trace!("message received from plugin: {:?}", message);
                executor.publish(message, ExecutorPublishOptions::default()).await;
            },
            Some(pubsub_message) = pubsub_stream.next() => {
                trace!("message received from pubsub");
                if let Ok(message) = pubsub_message {
                    if let Some(rpc_message) = executor.extract_pubsub_message(message) {
                        match rpc_message.data {
                            RPCMessageData::AssignChannels(channel_list) => {
                                info!("received assign channels rpc {:?}", channel_list);
                                let channel_count = channel_list.channels.len() as f64;
                                executor.set_channels(channel_list.channels).await;
                                TOTAL_CHANNELS.set(channel_count);
                            }
                            RPCMessageData::UpdateSettings() => {
                                match executor.read_settings::<S>().await {
                                    Ok(settings) => {
                                        plugin.set_settings(settings).await;
                                        trace!("settings updated correctly");
                                    },
                                    Err(err) => {
                                        warn!("failed to read settings {err}");
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}
