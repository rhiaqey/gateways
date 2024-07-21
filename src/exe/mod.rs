mod http;
mod metrics;

use crate::exe::http::start_private_http_server;
use futures::StreamExt;
use log::{debug, info, trace, warn};
use rhiaqey_common::env::parse_env;
use rhiaqey_common::executor::{Executor, ExecutorPublishOptions};
use rhiaqey_common::pubsub::{PublisherRegistrationMessage, RPCMessage, RPCMessageData};
use rhiaqey_sdk_rs::gateway::{Gateway, GatewayConfig};
use rhiaqey_sdk_rs::settings::Settings;
use tokio::signal;

use crate::exe::metrics::TOTAL_CHANNELS;

pub async fn run<P: Gateway<S> + Send + 'static, S: Settings>() {
    env_logger::init();
    let env = parse_env();

    let mut executor = match Executor::setup(env.clone()).await {
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
    let settings = executor
        .read_settings_async::<S>()
        .await
        .unwrap_or(S::default());

    let config = GatewayConfig {
        id: Some(executor.get_id()),
        name: Some(executor.get_name()),
        namespace: Some(executor.get_namespace()),
        organization: Some(executor.get_organization().to_string()),
        port: executor.get_public_port(),
        host: None,
    };

    let mut publisher_stream = match plugin.setup(config, Some(settings)).await {
        Err(error) => panic!("failed to setup publisher: {error}"),
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
        .expect("publisher must first register with hub");

    debug!("rpc registration message sent");

    plugin.start().await;

    tokio::spawn(start_private_http_server(port));

    let mut pubsub_stream = executor
        .create_hub_to_publishers_pubsub_async()
        .await
        .unwrap();

    let channel_count = executor.get_channel_count_async().await as i64;
    TOTAL_CHANNELS.set(channel_count);
    trace!("channel count is {channel_count}");

    info!("ready, set, go...");

    loop {
        tokio::select! {
            Ok(result) = signal::ctrl_c() => {
                trace!("signal caught: {:?}", result);
                break;
            },
            Some(message) = publisher_stream.recv() => {
                trace!("message received from plugin: {:?}", message);
                match executor.publish_async(message, ExecutorPublishOptions::default()).await {
                    Ok(size) => debug!("published to {size} channels"),
                    Err(err) => warn!("error publishing message: {}", err)
                }
            },
            Some(pubsub_message) = pubsub_stream.next() => {
                trace!("message received from hub");
                if let Ok(message) = pubsub_message {
                    if let Some(rpc_message) = executor.extract_pubsub_message(message) {
                        match rpc_message.data {
                            RPCMessageData::AssignChannels(channels) => {
                                debug!("received assign channels rpc {:?}", channels);
                                let channel_count = channels.len() as i64;
                                executor.set_channels_async(channels).await;
                                TOTAL_CHANNELS.set(channel_count);
                                info!("total channels assigned to {channel_count}");
                            }
                            RPCMessageData::UpdatePublisherSettings() => {
                                debug!("received update settings rpc");
                                match executor.read_settings_async::<S>().await {
                                    Ok(settings) => {
                                        plugin.set_settings(settings).await;
                                        info!("settings updated successfully");
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
    info!("shutting down");
}
