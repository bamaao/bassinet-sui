use std::{sync::Arc, time::Duration};
// use futures::StreamExt;
use sui_sdk::{rpc_types::{EventFilter, Page, SuiEvent}, types::{event::{EventID}, parse_sui_struct_tag}, SuiClient, SuiClientBuilder};
use tokio::time;

use crate::{events_mq::{publish_events, Config}, kv_store::{KVStore, RocksDB}};

/// 轮询查询事件
pub async  fn listening(package_id: &str, db: RocksDB, coinfig: Arc<Config>) -> Result<(), anyhow::Error>{
    loop {
        let client = get_client().await;
        if client.is_err() {
            let _ = time::sleep(Duration::from_secs(30));
            continue;
        }
        let client = client.unwrap();

        // 账户绑定事件
        let mut account_bound_event_id: Option<EventID> = Option::None;
        let accoount_bound_cursor = db.find("bassinet_account_bound");
        if accoount_bound_cursor.is_some() {
            account_bound_event_id = Some(EventID::try_from(accoount_bound_cursor.unwrap()).unwrap());
        }
        let account_bound_events = listening_account_bound_events(&client, package_id, account_bound_event_id, Option::Some(10)).await;
        if account_bound_events.is_err() {
            tracing::warn!("{:?}", account_bound_events.err());
        }else {
            let account_bound_events = account_bound_events.unwrap();
            // 事件发布到Rabbitmq
            if !account_bound_events.data.is_empty() {
                // for event in account_bound_events.data {
                //     tracing::info!("event_id:{},package_id:{},module:{},sender:{},type_:{},event:{}", String::from(event.id), event.package_id, event.transaction_module, event.sender, event.type_, serde_json::to_string_pretty(&event.parsed_json).unwrap());
                // }
                let _ = publish_events(coinfig.clone(), &account_bound_events.data, package_id, db.clone()).await;
            }
            // 存储游标
            if account_bound_events.has_next_page && account_bound_events.next_cursor.is_some() {
                db.save("bassinet_account_bound", String::from(account_bound_events.next_cursor.unwrap()).as_str());
            }
        }

        // 开通数字服务事件
        let mut service_opened_event_id: Option<EventID> = Option::None;
        let service_opened_cursor = db.find("bassinet_service_opened");
        if service_opened_cursor.is_some() {
            service_opened_event_id = Some(EventID::try_from(service_opened_cursor.unwrap()).unwrap());
        }
        let service_opened_events = listening_service_opened_events(&client, package_id, service_opened_event_id, Option::Some(10)).await;
        if service_opened_events.is_err() {
            tracing::warn!("{:?}", service_opened_events.err());
        }else {
            let service_opened_events = service_opened_events.unwrap();
            // 事件发布到Rabbitmq
            if !service_opened_events.data.is_empty() {
                let _ = publish_events(coinfig.clone(), &service_opened_events.data, package_id, db.clone()).await;
                // for event in service_opened_events.data {
                //     tracing::info!("event_id:{},package_id:{},module:{},sender:{},type_:{},event:{}", String::from(event.id), event.package_id, event.transaction_module, event.sender, event.type_, serde_json::to_string_pretty(&event.parsed_json).unwrap());
                // }
            }
            // 存储游标
            if service_opened_events.has_next_page && service_opened_events.next_cursor.is_some() {
                db.save("bassinet_service_opened", String::from(service_opened_events.next_cursor.unwrap()).as_str());
            }
        }

        // 发行NFT事件
        let mut nft_launched_event_id: Option<EventID> = Option::None;
        let nft_launched_cursor = db.find("bassinet_nft_launched");
        if nft_launched_cursor.is_some() {
            nft_launched_event_id = Some(EventID::try_from(nft_launched_cursor.unwrap()).unwrap());
        }
        let nft_launched_events = listening_nft_launched_events(&client, package_id, nft_launched_event_id, Option::Some(10)).await;
        if nft_launched_events.is_err() {
            tracing::warn!("{:?}", nft_launched_events.err());
        }else {
            let nft_launched_events = nft_launched_events.unwrap();
            // 事件发布到Rabbitmq
            if !nft_launched_events.data.is_empty() {
                // for event in nft_launched_events.data {
                //     tracing::info!("event_id:{},package_id:{},module:{},sender:{},type_:{},event:{}", String::from(event.id), event.package_id, event.transaction_module, event.sender, event.type_, serde_json::to_string_pretty(&event.parsed_json).unwrap());
                // }
                let _ = publish_events(coinfig.clone(), &nft_launched_events.data, package_id, db.clone()).await;
            }
            // 存储游标
            if nft_launched_events.has_next_page && nft_launched_events.next_cursor.is_some() {
                db.save("bassinet_nft_launched", String::from(nft_launched_events.next_cursor.unwrap()).as_str());
            }
        }
        // 暂停60s
        time::sleep(Duration::from_secs(60)).await;
    }
    Ok(())
}

// /// 订阅指定module的事件
// pub async fn subscribe(client: SuiClient, package_id: &str, module: &str) -> Result<(), anyhow::Error> {
//     let mut subscribe_all = client.event_api()
//     .subscribe_event(
//         EventFilter::MoveModule {
//             package: package_id.parse()?,
//             module: Identifier::new(module)?,
//         }
//     ).await?;

//     loop {
//         while let Some(event) = subscribe_all.next().await {
//             if event.is_err() {
//                 println!("Event error {:?}", event.err());
//             }else {
//                 println!("Event: {:?}", event.unwrap().parsed_json);
//             }
//         }
//     }
// }

pub async fn get_client() -> Result<SuiClient, anyhow::Error> {
    let client = SuiClientBuilder::default()
    // .ws_url("wss://sui-testnet-rpc.publicnode.com")
    // .build("https://sui-testnet-rpc.publicnode.com")
    .build("https://fullnode.testnet.sui.io:443")
    .await?;
    Ok(client)
}

/// 绑定账户事件
pub async fn listening_account_bound_events(client: &SuiClient, package_id: &str, event_id: Option<EventID>, limit: Option<usize>) -> Result<Page<SuiEvent, EventID>, anyhow::Error>{
    let mut tag_str = String::from(package_id);
    tag_str.push_str("::");
    tag_str.push_str("digital_service");
    tag_str.push_str("::AccountBound");
    let tag = parse_sui_struct_tag(tag_str.as_str()).unwrap();
    let struct_tag_filter = EventFilter::MoveEventType(tag);
    tracing::info!("{}", tag_str.as_str());

    let events = client
    .event_api()
    .query_events(
        struct_tag_filter,
        event_id,
        limit,
        false,
    )
    .await?;

    tracing::info!("接收{}条事件", events.data.len());

    Ok(events)
}

/// 监听开通数字服务事件
pub async fn listening_service_opened_events(client: &SuiClient, package_id: &str, event_id: Option<EventID>, limit: Option<usize>) -> Result<Page<SuiEvent, EventID>, anyhow::Error>{
    let mut tag_str = String::from(package_id);
    tag_str.push_str("::");
    tag_str.push_str("digital_service");
    tag_str.push_str("::DigitalServiceOpened");
    let tag = parse_sui_struct_tag(tag_str.as_str()).unwrap();
    let struct_tag_filter = EventFilter::MoveEventType(tag);

    let events = client
    .event_api()
    .query_events(
        struct_tag_filter,
        event_id,
        limit,
        false,
    )
    .await?;

    Ok(events)
}

/// 监听发行NFT事件
pub async fn listening_nft_launched_events(client: &SuiClient, package_id: &str, event_id: Option<EventID>, limit: Option<usize>) -> Result<Page<SuiEvent, EventID>, anyhow::Error>{
    let mut tag_str = String::from(package_id);
    tag_str.push_str("::");
    tag_str.push_str("launch_service");
    tag_str.push_str("::NftLaunched");
    let tag = parse_sui_struct_tag(tag_str.as_str()).unwrap();
    let struct_tag_filter = EventFilter::MoveEventType(tag);

    let events = client
    .event_api()
    .query_events(
        struct_tag_filter,
        event_id,
        limit,
        false,
    )
    .await?;

    Ok(events)
}