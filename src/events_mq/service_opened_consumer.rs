use std::{path::PathBuf, str::FromStr, sync::Arc};
use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{
        BasicAckArguments, BasicCancelArguments, BasicConsumeArguments, QueueBindArguments, QueueDeclareArguments
    },
    connection::{Connection, OpenConnectionArguments},
};
use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info };

// use super::RabbitError;

use crate::{events_mq::coin_published_producer, kv_store::{KVStore, RocksDB}, sui_service::digital_service::OpenDigitalServiceConfig};

use super::Config;

#[derive(Debug, Serialize, Deserialize)]
pub struct CoinPublishedMessage {
    pub package_id: String,
    pub treasury_lock_id: String,
    pub admin_cap_id: String,
    pub symbol: String,
    pub name: String,
    pub description: String,
    pub icon_url: String,
    pub account: String,
    pub wallet_address: String,
}

pub async fn service_opened_consume(cfg: Arc<Config>, db:RocksDB) -> anyhow::Result<()> {
    loop {
        let result = process(cfg.clone(), db.clone()).await;
        match result {
            Ok(value) => {
                // Not actually implemented right now.
                // warn!("exiting in response to a shutdown command");
                return Ok(value);
            }
            Err(err) => {
                error!("RabbitMQ connection returned error: {err:?}");
                sleep(Duration::from_millis(1000)).await;
                info!("ready to restart consumer task");
            }
        }
    }
}

async fn process(cfg: Arc<Config>, db:RocksDB) -> anyhow::Result<()> {
    debug!("starting service_opened task");

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

    // Declare our receive queue.
    let (queue_name, _, _) = channel
        .queue_declare(QueueDeclareArguments::durable_client_named("sui_service_opened_event").durable(true).exclusive(true).auto_delete(false).finish())
        .await
        .context("failed to declare queue")?
        .expect("when no_wait is false (default) then we should have a value");
    debug!("declared queue '{queue_name}'");

    let exchange_name = "bassinet.topic";
    debug!("binding exchange {exchange_name} -> queue {queue_name}");
    channel
        .queue_bind(QueueBindArguments::new(&queue_name, exchange_name, "bassinet.DigitalServiceOpened"))
        .await
        .context("queue binding failed")?;

    let consume_args = BasicConsumeArguments::new(&queue_name, "DigitalServiceOpened").auto_ack(false).finish();
    
    // let consumer = ServiceOpenedConsumer::new(consume_args.no_ack);
    // let consumer_tag = channel
    //     .basic_consume(consumer, consume_args)
    //     .await
    //     .context("failed basic_consume")?;
    // trace!("consumer tag: {consumer_tag}");

    let (ctag, mut rx) = channel.basic_consume_rx(consume_args).await.unwrap();
    let new_channel = channel.clone();
    let rocksdb = db.clone();
    let jh = tokio::spawn(async move {
        let dir_path = std::env::var("CONTRACTS_DIR_PATH").expect("CONTRACTS_DIR_PATH must be set");
        let provider = std::env::var("PROVIDER").expect("PROVIDER must be set");
        let key_store_path = std::env::var("KEY_STORE_PATH").expect("KEY_STORE_PATH must be set");
        while let Some(msg) = rx.recv().await {
            let content = msg.content.unwrap();
            let deliver = msg.deliver.unwrap();
            let json = std::str::from_utf8(&content).unwrap();
            info!(
                "consume delivery {}, content: {}",
                deliver,
                json
            );

            // 处理消息(public_key,address,symbol,name,description,icon_url)
            let value : Value = serde_json::from_str(json).unwrap();
            let address = value.get("address");
            let public_key = value.get("public_key");
            let symbol = value.get("symbol");
            let name = value.get("name");
            let description = value.get("description");
            let icon_url = value.get("icon_url");
            if address.is_some() && public_key.is_some() && symbol.is_some(){
                // pub account: String,
                // pub wallet_address: String,
                // pub dir: PathBuf,
                // pub symbol: String,
                // pub name: String,
                // pub description: String,
                // pub icon_url: String,
                // pub creator: String,
                // pub provider: String,
                // pub package_id: String
                let account = public_key.unwrap().as_str().unwrap().to_owned();
                let address = address.unwrap().as_str().unwrap();
                // let wallet_address = address.strip_prefix("0x").unwrap_or(address);
                let dir = PathBuf::from_str(&dir_path).unwrap();
                let symbol = symbol.unwrap().as_str().unwrap();
                let name = name.unwrap().as_str().unwrap();
                let description = description.unwrap().as_str().unwrap();
                let icon_url = icon_url.unwrap().as_str().unwrap();
                let creator = address;
                let package_id = "0x0";
                let mut config = OpenDigitalServiceConfig::new(
                    account.clone(), 
                    address.to_owned(),
                    dir,
                    symbol.to_owned(),
                    name.to_owned(),
                    description.to_owned(),
                    icon_url.to_owned(),
                    creator.to_owned(),
                    provider.to_owned(),
                    package_id.to_owned()
                );
                let result = config.open(&key_store_path).await;
                if result.is_err() {
                    tracing::error!("message:{}, error:{:?}", json, result.err());
                    // TODO 重大事件，其他通知方式
                }else {
                    // json序列化保存到rocksdb
                    let publishing_reslut = result.unwrap();
                    let package_id = publishing_reslut.package_id.clone();
                    let json = serde_json::to_string(&publishing_reslut).unwrap();
                    rocksdb.save(package_id.as_str(), json.as_str());
                    // 存储钱包地址对应的BassinetCoin的package_id
                    rocksdb.save(&(address.to_owned() + "_bassinet_coin"), package_id.as_str());

                    let message = CoinPublishedMessage{
                        package_id : package_id,
                        treasury_lock_id: publishing_reslut.treasury_lock_id,
                        admin_cap_id: publishing_reslut.admin_cap_id,
                        symbol: symbol.to_owned(),
                        name: name.to_owned(),
                        description: description.to_owned(),
                        icon_url: icon_url.to_owned(),
                        account: account,
                        wallet_address: address.to_owned()
                    };

                    // 发送mq消息
                    let _ = coin_published_producer::produce_coin_published(cfg.clone(), &message).await;
                }

                // Ack explicitly
                let args = BasicAckArguments::new(deliver.delivery_tag(), false);
                new_channel.basic_ack(args).await.unwrap();
            }
        }
    });
    assert!(jh.await.is_err());
    channel.basic_cancel(BasicCancelArguments::new(&ctag)).await.unwrap();
    Err(anyhow!("consumer panic"))
    // if connection.listen_network_io_failure().await {
    //     Err(RabbitError::ConnectionLost("connection failure".to_owned()).into())
    // } else {
    //     Err(RabbitError::ConnectionLost("connection shut down normally. Since we don't close it ourselves, this shouldn't happen in this program".to_owned()).into())
    // }
}

// pub struct ServiceOpenedConsumer {
//     no_ack: bool,
//     panic_countdown: u32,
// }

// impl ServiceOpenedConsumer {
//     /// Return a new consumer.
//     ///
//     /// See [Acknowledgement Modes](https://www.rabbitmq.com/consumers.html#acknowledgement-modes)
//     ///
//     /// no_ack = [`true`] means automatic ack and should NOT send ACK to server.
//     ///
//     /// no_ack = [`false`] means manual ack, and should send ACK message to server.
//     pub fn new(no_ack: bool) -> Self {
//         Self {
//             no_ack,
//             panic_countdown: 2,
//         }
//     }
// }

// #[async_trait]
// impl AsyncConsumer for ServiceOpenedConsumer {
//     async fn consume(
//         &mut self,
//         channel: &Channel,
//         deliver: Deliver,
//         _basic_properties: BasicProperties,
//         content: Vec<u8>,
//     ) {
//         info!(
//             "consume delivery {} on channel {}, content size: {}, content: {}",
//             deliver,
//             channel,
//             content.len(),
//             String::from_utf8(content).unwrap()
//         );

//         // match self.panic_countdown {
//         //     0 => {
//         //         self.panic_countdown = 2;
//         //         info!("panic time!");
//         //         panic!("testing consumer handling of panics");
//         //     }
//         //     i => {
//         //         info!("panic countdown: {i}");
//         //         self.panic_countdown -= 1;
//         //     }
//         // };

//         // Ack explicitly if using manual ack mode. Otherwise, the library auto-acks it.
//         if !self.no_ack {
//             info!("ack to delivery {} on channel {}", deliver, channel);
//             let args = BasicAckArguments::new(deliver.delivery_tag(), false);
//             channel.basic_ack(args).await.unwrap();
//         }
//     }
// }