use std::{collections::HashMap, path::PathBuf, str::FromStr, sync::Arc};
use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{
        BasicAckArguments, BasicCancelArguments, BasicConsumeArguments, QueueBindArguments, QueueDeclareArguments
    },
    connection::{Connection, OpenConnectionArguments},
};
use anyhow::{anyhow, Context};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{de, Value};
use sui_sdk::types::base_types::ObjectID;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

// use super::RabbitError;

use crate::{events_mq::nft_published_producer, kv_store::{KVStore, RocksDB}, sui_service::{nft_service::{NftConfigInfo, NftServiceConfig}, BassinetCoinPublishedResult}};

use super::Config;

#[derive(Debug, Serialize, Deserialize)]
pub struct NftPublishedMessage {
    pub collection_id: String,
    pub package_id: String,
    pub mint_id: String,
    pub policy_id: String,
    pub policy_cap_id: String,
    pub coin_package_id: String,
    pub treasury_lock_id: String,
    pub admin_cap_id: String,
    pub description: String,
    pub collection_url: String,
    pub limit: u64,
    pub rewards_quantity: u64,
    pub minting_price: u64,
}

pub async fn nft_launched_consume(cfg: Arc<Config>, db:RocksDB) -> anyhow::Result<()> {
    loop {
        let result = process(cfg.clone(), &db).await;
        match result {
            Ok(value) => {
                // Not actually implemented right now.
                warn!("exiting in response to a shutdown command");
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

async fn process(cfg: Arc<Config>, db:&RocksDB) -> anyhow::Result<()> {
    debug!("starting nft_launched task");

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
        .queue_declare(QueueDeclareArguments::durable_client_named("sui_nft_launched_event").durable(true).exclusive(true).auto_delete(false).finish())
        .await
        .context("failed to declare queue")?
        .expect("when no_wait is false (default) then we should have a value");
    debug!("declared queue '{queue_name}'");

    let exchange_name = "bassinet.topic";
    debug!("binding exchange {exchange_name} -> queue {queue_name}");
    channel
        .queue_bind(QueueBindArguments::new(&queue_name, exchange_name, "bassinet.NftLaunched"))
        .await
        .context("queue binding failed")?;

    let consume_args = BasicConsumeArguments::new(&queue_name, "NftLaunched").auto_ack(false).finish();
    // let consumer = NftLaunchedConsumer::new(consume_args.no_ack);
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
        let host = std::env::var("HOST").expect("HOST must be set");
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
            // 处理消息(public_key,address,collection_id,limit,rewards_quantity,minting_price)
            let value : Value = serde_json::from_str(json).unwrap();
            let public_key = value.get("public_key").unwrap().as_str().unwrap();
            let address = value.get("address").unwrap().as_str().unwrap();
            let collection_id = value.get("collection_id").unwrap().as_str().unwrap();
            let limit_str = value.get("limit").unwrap().as_str().unwrap();
            let limit = limit_str.parse::<u64>().unwrap();
            let rewards_quantity_str = value.get("rewards_quantity").unwrap().as_str().unwrap();
            let rewards_quantity = rewards_quantity_str.parse::<u64>().unwrap();
            let minting_price_str = value.get("minting_price").unwrap().as_str().unwrap();
            let minting_price = minting_price_str.parse::<u64>().unwrap();
            let dir = PathBuf::from_str(&dir_path).unwrap();
            let creator = address;
            let package_id = "0x0";
            // 从RocksDB中获取
            let coin_package_id = rocksdb.find(&(address.to_owned() + "_bassinet_coin"));
            if coin_package_id.is_none() {
                tracing::error!("message:{}, error:Bassinet Coin package id not exist", json);
                // TODO 重大事件，其他通知方式
            }else {
                let coin_package_id = coin_package_id.unwrap();
                let coin_info = rocksdb.find(&coin_package_id).unwrap();

                let bassinet_coin: BassinetCoinPublishedResult = serde_json::from_str(&coin_info).unwrap();
                // 获取collection_id信息
                let collection_info = get_collection(collection_id, host.as_str()).await;
                if collection_info.is_err() {
                    tracing::error!("message:{}, error:{:?}", json, collection_info.err());
                    // TODO 重大事件，其他通知方式
                }else {
                    let (collection_url, description) = collection_info.unwrap();
                    let mut config = NftServiceConfig {
                        account: public_key.to_owned(),
                        wallet_address: address.to_owned(),
                        dir: dir,
                        collection_id: collection_id.to_owned(),
                        creator: creator.to_owned(),
                        provider: provider.to_owned(),
                        coin_package_id: coin_package_id.to_owned(),
                        package_id: package_id.to_owned(),
                    };
                    // 发布NFT
                    let published_result = config.launch(&key_store_path).await;
                    if published_result.is_err() {
                        tracing::error!("message:{}, error:{:?}", json, published_result.err());
                        // TODO 重大事件，其他通知方式
                    }else {
                        // json序列化保存到rocksdb
                        let publishing_reslut = published_result.unwrap();
                        let package_id = publishing_reslut.package_id.clone();
                        let json = serde_json::to_string(&publishing_reslut).unwrap();
                        rocksdb.save(package_id.as_str(), json.as_str());
                        // collection_id对应的NFT package_id
                        rocksdb.save(collection_id, package_id.as_str());
        
                        let config_info = NftConfigInfo{
                            description: description.to_owned(),
                            collection_id: collection_id.to_owned(),
                            collection_url: collection_url.to_owned(),
                            limit: limit,
                            rewards_quantity: rewards_quantity,
                            minting_price: minting_price,
                        };
                        let policy_id = ObjectID::from_hex_literal(&publishing_reslut.policy_id).unwrap();
                        let mint_id = ObjectID::from_hex_literal(&publishing_reslut.mint_id).unwrap();
                        // 初始配置NFT
                        let init_result = config.init_config(&config_info, policy_id, mint_id, &key_store_path).await;
                        if init_result.is_err() {
                            tracing::error!("初始化配置:message:{}, package_id:{}, error:{:?}", json, package_id, init_result.err());
                            // TODO 重大事件，其他通知方式
                        }
    
                        let message = NftPublishedMessage {
                            collection_id: collection_id.to_owned(),
                            package_id: package_id,
                            mint_id: publishing_reslut.mint_id,
                            policy_id: publishing_reslut.policy_id,
                            policy_cap_id: publishing_reslut.policy_cap_id,
                            coin_package_id: bassinet_coin.package_id,
                            treasury_lock_id: bassinet_coin.treasury_lock_id,
                            admin_cap_id: bassinet_coin.admin_cap_id,
                            description: description,
                            collection_url: collection_url,
                            limit: limit,
                            rewards_quantity: rewards_quantity,
                            minting_price: minting_price,
                        };
    
                        // 发送mq消息
                        let _ = nft_published_producer::produce_nft_published(cfg.clone(), &message).await;
                    }
                }
            }
            // Ack explicitly
            let args = BasicAckArguments::new(deliver.delivery_tag(), false);
            new_channel.basic_ack(args).await.unwrap();
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

async fn get_collection(collection_id: &str, host: &str) -> Result<(String, String), anyhow::Error> {
    let mut count = 0;
    while count < 60 {
        count += 1;
        let result = get_collection_simple_info(collection_id, host).await;
        match result {
            Ok(value) => {
                return Ok(value);
            }
            Err(err) => {
                error!("Can't Get Collection:{} Info", collection_id);
                sleep(Duration::from_secs(10)).await;
            }
        }
    }
    Err(anyhow!(format!("Can't Get Collection:{} Info", collection_id)))
}
 
async fn get_collection_simple_info(collection_id: &str, host: &str) -> Result<(String, String), anyhow::Error> {
    let url = host.to_owned() + "/collections/" + collection_id + "/simpleinfo";
    let resp = reqwest::get(url).await;
    // println!("{resp:#?}");
    if resp.is_err() {
        return Err(anyhow!(resp.err().unwrap().to_string()))
    }

    let response = resp.unwrap();
    if response.status() != StatusCode::OK {
        return Err(anyhow!("No Collection Info"))
    }
    
    let values = response.json::<HashMap<String, String>>().await;
    if values.is_err() {
        return Err(anyhow!("Invalid Collection Info"))
    }

    let values = values.unwrap();

    let description_opt = values.get("title");
    if description_opt.is_none() {
        return Err(anyhow!("Invalid Collection Info"))
    }
    let  description  = urlencoding::encode(description_opt.unwrap().as_str()).into_owned();
    
    let collection_url_opt = values.get("collection_url");
    if collection_url_opt.is_none() {
        return Err(anyhow!("Invalid Collection Info"))
    }

    Ok((collection_url_opt.unwrap().to_string(), description))
}

// pub struct NftLaunchedConsumer {
//     no_ack: bool,
//     panic_countdown: u32,
// }

// impl NftLaunchedConsumer {
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
// impl AsyncConsumer for NftLaunchedConsumer {
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