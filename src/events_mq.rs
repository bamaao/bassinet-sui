use std::{env, sync::Arc};
use sui_sdk::rpc_types::SuiEvent;

use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{BasicPublishArguments, ExchangeDeclareArguments, ExchangeDeleteArguments},
    connection::{Connection, OpenConnectionArguments},
    BasicProperties
};
use anyhow::{Context};
use thiserror::Error;
use tokio::time::{self, sleep, Duration};
use tracing::{debug, error, info};

use crate::kv_store::{KVStore, RocksDB};

pub mod service_opened_consumer;
pub mod nft_launched_consumer;
pub mod coin_published_producer;
pub mod nft_published_producer;

/// Load the application configuration.
/// Uses environment variable, but in reality it might use some other external configuration source.
pub async fn load_config() -> Config {
    sleep(Duration::from_millis(10)).await; // delay to simulate loading configuration
    Config {
        virtual_host: env::var("RABBIT_VHOST").unwrap_or("/".to_owned()),
        host: env::var("RABBIT_HOST").unwrap_or("localhost".to_owned()),
        password: env::var("RABBIT_PASSWORD").unwrap_or("guest".to_owned()),
        port: env::var("RABBIT_PORT")
            .map(|s| s.parse::<u16>().expect("can't parse RABBIT_PORT"))
            .unwrap_or(5672),
        username: env::var("RABBIT_USER").unwrap_or("guest".to_owned()),
    }
}

// pub async fn shutdown_monitor(cfg: Arc<Config>) -> anyhow::Result<()> {
//     // Show how tasks can share access to application config, though obviously we don't need config here right now.
//     info!(
//         "waiting for Ctrl+c.  I have access to the configuration. Rabbit host: {}",
//         cfg.host
//     );
//     tokio::signal::ctrl_c()
//         .await
//         .context("problem waiting for ctrl+c")?;
//     info!("received Ctrl+c signal");
//     Ok(())
// }

/// Application configuration data.
#[derive(Debug)]
pub struct Config {
    pub virtual_host: String,
    pub host: String,
    pub password: String,
    pub port: u16,
    pub username: String,
}

#[derive(Error, Debug)]
pub enum RabbitError {
    #[error("RabbitMQ server connection lost: {0}")]
    ConnectionLost(String),
}

pub async fn declare_exchange(cfg: Arc<Config>) -> anyhow::Result<()> {
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
    channel
        .exchange_declare(ExchangeDeclareArguments::new(exchange_name, "topic").durable(true).finish())
        .await
        .context("declare exchange failed")?;
    // explicitly close
    channel.close().await.unwrap();
    connection.close().await.unwrap();

    Ok(())
}

pub async fn delete_exchange(cfg: Arc<Config>) -> anyhow::Result<()> {
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
    channel
        .exchange_delete(ExchangeDeleteArguments::new(exchange_name))
        .await
        .context("delete exchange failed")?;
    // explicitly close
    channel.close().await.unwrap();
    connection.close().await.unwrap();

    Ok(())
}

pub async fn publish_events(cfg: Arc<Config>, events: &Vec<SuiEvent>, package_id: &str, db: RocksDB) -> anyhow::Result<()> {
    loop {
        let result = process(cfg.clone(), &events, package_id, db.clone()).await;
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

pub async fn process(cfg: Arc<Config>, events: &Vec<SuiEvent>, package_id: &str, db: RocksDB) -> anyhow::Result<()> {
    debug!("starting producer task");

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
    for event in events {
        let event_type = event_type(event, package_id);
        if event_type.is_none() {
            continue;
        }
        if event_exists(&event, &db) {
            continue;
        }
        let routing_key = "bassinet.".to_owned() + &event_type.unwrap();
        // create arguments for basic_publish
        let args = BasicPublishArguments::new(exchange_name, routing_key.as_str());
        let content = serde_json::to_string_pretty(&event.parsed_json).unwrap();
        channel
        .basic_publish(
            BasicProperties::default().with_persistence(true).finish(),
            content.as_bytes().to_vec(),
            args,
        )
        .await
        .unwrap();
        mark_event(&event, &db);
        tracing::info!("发布事件:{}, routing_key:{}", content, routing_key);
    }

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

fn event_type(event: &SuiEvent, package_id: &str) -> Option<String> {
    if event.package_id.to_string().as_str() == package_id {
        let event_type = event.type_.name.as_str();
        if event_type == "AccountBound" {
            return Some("AccountBound".to_owned())
        }else if event_type == "DigitalServiceOpened" {
            return Some("DigitalServiceOpened".to_owned())
        }else if event_type == "NftLaunched" {
            return Some("NftLaunched".to_owned())
        }
    }
    Option::None
}

fn event_exists(event: &SuiEvent, db: &RocksDB) -> bool {
    // let event_type = event.type_.name.as_str();
    // if event_type == "NftLaunched" {
    //     return false
    // }
    db.find(String::from(event.id).as_str()).is_some()
}

fn mark_event(event: &SuiEvent, db: &RocksDB) {
    db.save(String::from(event.id).as_str(), "1");
}

// pub struct MyConsumer {
//     no_ack: bool,
// }

// impl MyConsumer {
//     /// Return a new consumer.
//     ///
//     /// See [Acknowledgement Modes](https://www.rabbitmq.com/consumers.html#acknowledgement-modes)
//     ///
//     /// no_ack = [`true`] means automatic ack and should NOT send ACK to server.
//     ///
//     /// no_ack = [`false`] means manual ack, and should send ACK message to server.
//     pub fn new(no_ack: bool) -> Self {
//         Self { no_ack }
//     }
// }

// #[async_trait]
// impl AsyncConsumer for MyConsumer {
//     async fn consume(
//         &mut self,
//         channel: &Channel,
//         deliver: Deliver,
//         _basic_properties: BasicProperties,
//         content: Vec<u8>,
//     ) {
//         info!(
//             "consume delivery {} on channel {}, content size: {}",
//             deliver,
//             channel,
//             content.len()
//         );

//         // Ack explicitly if using manual ack mode. Otherwise, the library auto-acks it.
//         if !self.no_ack {
//             info!("ack to delivery {} on channel {}", deliver, channel);
//             let args = BasicAckArguments::new(deliver.delivery_tag(), false);
//             channel.basic_ack(args).await.unwrap();
//         }
//     }
// }