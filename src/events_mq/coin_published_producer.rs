use std::{sync::Arc};

use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{BasicPublishArguments},
    connection::{Connection, OpenConnectionArguments},
    BasicProperties
};
use anyhow::{Context};
use tokio::time::{self, sleep, Duration};
use tracing::{debug, error, info};

use super::{service_opened_consumer::CoinPublishedMessage, Config};


pub async fn produce_coin_published(cfg: Arc<Config>, msg: &CoinPublishedMessage) -> anyhow::Result<()> {
    loop {
        let result = process(cfg.clone(), msg).await;
        match result {
            Ok(value) => {
                // Not actually implemented right now.
                // warn!("exiting in response to a shutdown command");
                return Ok(value);
            }
            Err(err) => {
                error!("RabbitMQ connection returned error: {err:?}");
                sleep(Duration::from_millis(1000)).await;
                info!("ready to restart RabbitMQ task");
            }
        }
    }
}

pub async fn process(cfg: Arc<Config>, msg: &CoinPublishedMessage) -> anyhow::Result<()> {
    debug!("starting coin_published_producer task");

    let connection = Connection::open(
        &OpenConnectionArguments::new(&cfg.host, cfg.port, &cfg.username, &cfg.password)
            .virtual_host(&cfg.virtual_host),
    )
    .await
    .with_context(|| {
        format!(
            "can't connect to RabbitMQ server at {}:{}",
            cfg.host, cfg.port
        )
    })?;

    // Add simple connection callback, it just logs diagnostics.
    connection
        .register_callback(DefaultConnectionCallback)
        .await
        .context("registering connection callback failed")?;

    let channel = connection
        .open_channel(None)
        .await
        .context("opening channel failed")?;
    channel
        .register_callback(DefaultChannelCallback)
        .await
        .context("registering channel callback failed")?;
    let exchange_name = "bassinet.topic";
    
    // 发送事件
    let routing_key = "bassinet.CoinPublished";
    let json = serde_json::to_string(msg).unwrap();
    // create arguments for basic_publish
    let args = BasicPublishArguments::new(exchange_name, routing_key);
    channel
    .basic_publish(
        BasicProperties::default().with_persistence(true).finish(),
        json.as_bytes().to_vec(),
        args,
    )
    .await
    .unwrap();

    // keep the `channel` and `connection` object from dropping before pub/sub is done.
    // channel/connection will be closed when drop.
    time::sleep(time::Duration::from_secs(10)).await;
    // explicitly close
    channel.close().await.unwrap();
    connection.close().await.unwrap();

    Ok(())

    // if connection.listen_network_io_failure().await {
    //     Err(RabbitError::ConnectionLost("connection failure".to_owned()).into())
    // } else {
    //     Err(RabbitError::ConnectionLost("connection shut down normally. Since we don't close it ourselves, this shouldn't happen in this program".to_owned()).into())
    // }
}