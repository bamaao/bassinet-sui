use std::{collections::HashMap, sync::Arc, time::Duration};

use event_listening::listening;
use kv_store::{KVStore, RocksDB};
use events_mq::{load_config, nft_launched_consumer::nft_launched_consume, service_opened_consumer::service_opened_consume};
use reqwest::StatusCode;
use tokio::time::sleep;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing::{error, info};
use anyhow::{anyhow};

mod event_listening;
mod sui_service;
mod ed25519;
mod sui_ed25519;
mod events_mq;
mod archive;
mod template;
mod sui_api_integration;
mod kv_store;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenvy::dotenv()?;

    // tracing_subscriber::fmt::init();
    tracing_subscriber::registry()
    .with(
        tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
    ).with(tracing_subscriber::fmt::layer())
    .init();

    // let config = Arc::new(load_config().await);
    // println!("config:{:?}", config.clone());
    // // 删除exchange
    // let _ = delete_exchange(config.clone()).await;
    // // 声明exchange
    // let _ = declare_exchange(config.clone()).await;

    let rocksdb_dir_path = std::env::var("ROCKSDB_STORE_DIR_PATH").expect("ROCKSDB_STORE_DIR_PATH must be set");
    let db = RocksDB::init(rocksdb_dir_path.as_str());

    let config = Arc::new(load_config().await);
    println!("config:{:?}", config.clone());

    let service_opened_cfg = config.clone();
    tokio::spawn(service_opened_consume(service_opened_cfg, db.clone()));

    let nft_launched_cfg = config.clone();
    tokio::spawn(nft_launched_consume(nft_launched_cfg, db.clone()));

    // let package_id = "0x4b02907c0d7f471048c98e318343a0ed29b6e5e3a505bcf894106a9b2a915ac5";
    let package_id = std::env::var("LISTENING_PACKAGE_ID").expect("LISTENING_PACKAGE_ID must be set");
    let _= listening(package_id.as_str(), db.clone(), config.clone()).await;

    // let host = std::env::var("HOST").expect("HOST must be set");
    // let collection_id = uuid::Uuid::new_v4().to_string();
    // let result = get_collection(collection_id.as_str(), host.as_str()).await.unwrap();
    // println!("collection_url:{}, description:{}", result.0, result.1);

    Ok(())
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
                sleep(Duration::from_millis(1000)).await;
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