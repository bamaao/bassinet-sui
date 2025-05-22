use std::{path::PathBuf, str::FromStr};
use anyhow::{anyhow, Ok};
use serde::{Deserialize, Serialize};
use shared_crypto::intent::Intent;
use sui_keys::keystore::{AccountKeystore, FileBasedKeystore};
use sui_sdk::{rpc_types::{Coin, ObjectChange, SuiObjectData, SuiObjectDataFilter, SuiObjectDataOptions, SuiObjectResponseQuery, SuiTransactionBlockEffectsAPI, SuiTransactionBlockResponseOptions}, types::{base_types::{ObjectID, SuiAddress}, parse_sui_struct_tag, programmable_transaction_builder::ProgrammableTransactionBuilder, quorum_driver_types::ExecuteTransactionRequestType, transaction::{Argument, CallArg, Command, ObjectArg, Transaction, TransactionData}, Identifier}, SuiClientBuilder};

use digital_service::OpenDigitalServiceConfig;
use nft_service::{NftConfigInfo, NftServiceConfig};

pub mod digital_service;
pub mod nft_service;

#[derive(Debug, Serialize, Deserialize)]
pub struct BassinetCoinPublishedResult {
    pub package_id: String,
    pub admin_cap_id: String,
    pub treasury_lock_id: String,
    pub wallet_address: String,
    pub account: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NftPublishedResult {
    pub collection_id: String,
    pub package_id: String,
    pub mint_id: String,
    pub policy_id: String,
    pub policy_cap_id: String,
}

/// 发布代币合约
/// "D:/Users/zouyc/.sui/sui_config/sui.keystore"
pub async fn publish(config: &OpenDigitalServiceConfig, modules: Vec<Vec<u8>>, dependencies: Vec<ObjectID>, key_store_path: &str) -> Result<BassinetCoinPublishedResult, anyhow::Error> {
    let sui_test = SuiClientBuilder::default()
        // .ws_url("wss://sui-testnet-rpc.publicnode.com")
        // .build("https://sui-testnet-rpc.publicnode.com")
        .build("https://fullnode.testnet.sui.io:443")
        .await?;

    let mut ptb = ProgrammableTransactionBuilder::new();
    let provider = config.provider.strip_prefix("0x").unwrap_or(config.provider.as_str());
    let sender = SuiAddress::from_bytes(hex::decode(&provider).unwrap()).unwrap();

    // we need to find the coin we will use as gas
    let coins = sui_test
        .coin_read_api()
        .get_coins(sender, None, None, None)
        .await?;
    let coin = coins.data.into_iter().next().unwrap();
    // let mut gas_coin : Option<Coin> = Option::None;
    // let mut iter = coins.data.into_iter();

    // let mut payment: Option<Coin> = Option::None;
    // let paid = 1_000_000_000u64;
    // while let Some(coin) = iter.next() {
    //     if coin.balance >= paid {
    //         payment = Some(coin);
    //     }else {
    //         gas_coin = Some(coin);
    //     }
    // }

    // ptb.command(Command::move_call(package, module, function, vec![], vec![Argument::Input(0), Argument::Input(1)]));
    ptb.command(Command::Publish(modules, dependencies));

    // upgradable
    let argument_address = ptb.pure(sender)?;
    ptb.command(Command::TransferObjects(vec![Argument::Result(0)], argument_address));

    let builder = ptb.finish();
    let gas_budget = 900_000_000;
    let gas_price = sui_test.read_api().get_reference_gas_price().await?;

    // create the transaction data that will be sent to the network
    let tx_data = TransactionData::new_programmable(
        sender,
        // vec![gas_coin.unwrap().object_ref()],
        vec![coin.object_ref()],
        builder,
        gas_budget,
        gas_price,
    );

    // 4) sign transaction
    let keystore = FileBasedKeystore::new(&PathBuf::from_str(key_store_path).unwrap())?;
    let signature = keystore.sign_secure(&sender, &tx_data, Intent::sui_transaction())?;

    // 5) execute the transaction
    print!("Executing the transaction...");
    let transaction_response = sui_test
        .quorum_driver_api()
        .execute_transaction_block(
            Transaction::from_data(tx_data, vec![signature]),
            SuiTransactionBlockResponseOptions::full_content(),
            Some(ExecuteTransactionRequestType::WaitForLocalExecution),
        )
        .await?;
    println!("{}", transaction_response);
    let status = transaction_response.status_ok();
    let mut id:Option<ObjectID> =  Option::None;
    let mut admin_cap: Option<ObjectID> = Option::None;
    let mut treasury_lock: Option<ObjectID> = Option::None;
    if status.is_some() && status.unwrap() == true {
        if transaction_response.object_changes.is_some() {
            for item in transaction_response.object_changes.unwrap().into_iter() {
                match item {
                    ObjectChange::Published{ package_id, version: _, digest: _, modules: _ } => {id = Option::Some(package_id)},
                    ObjectChange::Created { sender:_, owner:_, object_type, object_id, version:_, digest:_ } => {
                        if object_type.name.as_str() == "AdminCap" {
                            admin_cap = Some(object_id);
                        }else if object_type.name.as_str() == "TreasuryLock" {
                            treasury_lock = Some(object_id);
                        }
                    }
                    _ => {}
                }
            }
        }
        println!("package_id:{:?},admin_cap_id:{:?},treasury_lock_id:{:?}", id.unwrap().to_hex_literal(), admin_cap.unwrap().to_hex_literal(), treasury_lock.unwrap().to_hex_literal());
    }else {
        let message = format!("{}", transaction_response.effects.unwrap().into_status());
        return Err(anyhow!(message))
    }
    
    Ok(BassinetCoinPublishedResult { package_id: id.unwrap().to_hex_literal(), admin_cap_id: admin_cap.unwrap().to_hex_literal(), treasury_lock_id: treasury_lock.unwrap().to_hex_literal(), wallet_address: config.wallet_address.clone(), account: config.account.clone()})
}

/// 发布NFT代币合约
pub async fn publish_nft(config: &NftServiceConfig, modules: Vec<Vec<u8>>, dependencies: Vec<ObjectID>, key_store_path: &str) -> Result<NftPublishedResult, anyhow::Error> {
    let sui_test = SuiClientBuilder::default()
        // .ws_url("wss://sui-testnet-rpc.publicnode.com")
        // .build("https://sui-testnet-rpc.publicnode.com")
        .build("https://fullnode.testnet.sui.io:443")
        .await?;

    let mut ptb = ProgrammableTransactionBuilder::new();
    let provider = config.provider.strip_prefix("0x").unwrap_or(config.provider.as_str());
    let sender = SuiAddress::from_bytes(hex::decode(&provider).unwrap()).unwrap();

    // we need to find the coin we will use as gas
    let coins = sui_test
        .coin_read_api()
        .get_coins(sender, None, None, None)
        .await?;
    let coin = coins.data.into_iter().next().unwrap();
    // let mut gas_coin : Option<Coin> = Option::None;
    // let mut iter = coins.data.into_iter();

    // let mut payment: Option<Coin> = Option::None;
    // let paid = 1_000_000_000u64;
    // while let Some(coin) = iter.next() {
    //     if coin.balance >= paid {
    //         payment = Some(coin);
    //     }else {
    //         gas_coin = Some(coin);
    //     }
    // }

    // ptb.command(Command::move_call(package, module, function, vec![], vec![Argument::Input(0), Argument::Input(1)]));
    ptb.command(Command::Publish(modules, dependencies));

    // upgradable
    let argument_address = ptb.pure(sender)?;
    ptb.command(Command::TransferObjects(vec![Argument::Result(0)], argument_address));

    let builder = ptb.finish();
    let gas_budget = 900_000_000;
    let gas_price = sui_test.read_api().get_reference_gas_price().await?;

    // create the transaction data that will be sent to the network
    let tx_data = TransactionData::new_programmable(
        sender,
        // vec![gas_coin.unwrap().object_ref()],
        vec![coin.object_ref()],
        builder,
        gas_budget,
        gas_price,
    );

    // 4) sign transaction
    let keystore = FileBasedKeystore::new(&PathBuf::from_str(key_store_path).unwrap())?;
    let signature = keystore.sign_secure(&sender, &tx_data, Intent::sui_transaction())?;

    // 5) execute the transaction
    print!("Executing the transaction...");
    let transaction_response = sui_test
        .quorum_driver_api()
        .execute_transaction_block(
            Transaction::from_data(tx_data, vec![signature]),
            SuiTransactionBlockResponseOptions::full_content(),
            Some(ExecuteTransactionRequestType::WaitForLocalExecution),
        )
        .await?;
    println!("{}", transaction_response);
    let status = transaction_response.status_ok();
    let mut id: Option<ObjectID> =  Option::None;
    let mut mint_id: Option<ObjectID> = Option::None;
    let mut policy_id: Option<ObjectID> = Option::None;
    let mut policy_cap_id: Option<ObjectID> = Option::None;
    if status.is_some() && status.unwrap() == true {
        if transaction_response.object_changes.is_some() {
            for item in transaction_response.object_changes.unwrap().into_iter() {
                match item {
                    ObjectChange::Published{ package_id, version: _, digest: _, modules: _ } => {id = Option::Some(package_id)},
                    ObjectChange::Created { sender:_, owner:_, object_type, object_id, version:_, digest:_ } => {
                        if object_type.name.as_str() == "Mint" {
                            mint_id = Some(object_id);
                        }else if object_type.name.as_str() == "TransferPolicyCap" {
                            policy_cap_id = Some(object_id);
                        }else if object_type.name.as_str() == "TransferPolicy" {
                            policy_id = Some(object_id);
                        }
                    }
                    _ => {}
                }
            }
        }
        println!("package_id:{:?}, mint_id:{:?}, policy_id:{:?}, policy_cap_id:{:?}", id.unwrap().to_hex_literal(), mint_id.unwrap().to_hex_literal(), policy_id.unwrap().to_hex_literal(), policy_cap_id.unwrap().to_hex_literal());
    }else {
        let message = format!("{}", transaction_response.effects.unwrap().into_status());
        return Err(anyhow!(message))
    }
    let result = NftPublishedResult {
        collection_id: config.collection_id.clone(),
        package_id: id.unwrap().to_hex_literal(),
        mint_id: mint_id.unwrap().to_hex_literal(),
        policy_id: policy_id.unwrap().to_hex_literal(),
        policy_cap_id: policy_cap_id.unwrap().to_hex_literal(),
    };
    Ok(result)
}

/// 初始配置NFT合约
pub async fn init_config_nft(config: &NftServiceConfig, nft_config: &NftConfigInfo, policy_id: ObjectID, mint_id: ObjectID, key_store_path: &str) -> Result<(), anyhow::Error> {
    let sui_test = SuiClientBuilder::default()
        // .ws_url("wss://sui-testnet-rpc.publicnode.com")
        // .build("https://sui-testnet-rpc.publicnode.com")
        .build("https://fullnode.testnet.sui.io:443")
        .await?;

    let mut ptb = ProgrammableTransactionBuilder::new();
    let provider = config.provider.strip_prefix("0x").unwrap_or(config.provider.as_str());
    let sender = SuiAddress::from_bytes(hex::decode(&provider).unwrap()).unwrap();

    // we need to find the coin we will use as gas
    let coins = sui_test
        .coin_read_api()
        .get_coins(sender, None, None, None)
        .await?;
    // let coin = coins.data.into_iter().next().unwrap();
    let mut gas_coin : Option<Coin> = Option::None;
    let mut iter = coins.data.into_iter();

    let paid = 1_000_000_000u64;
    while let Some(coin) = iter.next() {
        if coin.balance >= paid {
            gas_coin = Some(coin);
        }
    }

    let package = ObjectID::from_hex_literal(&config.package_id).map_err(|e| anyhow!(e))?;
    let module = Identifier::new("bassinet").map_err(|e| anyhow!(e))?;
    let function = Identifier::new("authorize").map_err(|e| anyhow!(e))?;

    // entry fun authorize(
    //     admin_cap: &AdminCap,
    //     self: &mut Mint,
    //     policy: &mut TransferPolicy<BassinetNFT>,
    //     policy_cap: &TransferPolicyCap<BassinetNFT>,
    //     app_name: String,
    //     description: vector<u8>,
    //     collection_id: vector<u8>,
    //     collection_url: vector<u8>,
    //     limit: u64,
    //     rewards_quantity: u64,
    //     minting_price: u64
    // )

    // admin_cap owned
    let admin_cap_type = config.coin_package_id.clone() + "::bassinet_coin::AdminCap";
    let admin_cap = get_owned_object(admin_cap_type, sender, ObjectID::from_hex_literal(config.coin_package_id.as_str()).unwrap(), "bassinet_coin".to_owned()).await;
    if admin_cap.is_err() {
        return Err(anyhow!(admin_cap.err().unwrap().to_string()))
    }
    let admin_cap_object = admin_cap.unwrap();
    let admin_cap_arg = CallArg::Object(ObjectArg::ImmOrOwnedObject(admin_cap_object.object_ref()));
    ptb.input(admin_cap_arg).unwrap();

    // mint share
    let mint = get_object(mint_id).await;
    if mint.is_err() {
        return Err(anyhow!(mint.err().unwrap().to_string()))
    }
    let mint_object = mint.unwrap();
    let mint_arg = CallArg::Object(ObjectArg::SharedObject{
        id: mint_object.object_id,
        initial_shared_version: mint_object.version,
        mutable: true,
    });
    ptb.input(mint_arg).unwrap();

    // policy share 0x2::transfer_policy::TransferPolicy<0xbe96c8adaab4785c4ff5e383cacdef0e74e7b72cac5773c234e84c21298029b1::bassinet_nft::BassinetNFT>
    let policy = get_object(policy_id).await;
    if policy.is_err() {
        return Err(anyhow!(policy.err().unwrap().to_string()))
    }
    let policy_object = policy.unwrap();
    let policy_arg = CallArg::Object(ObjectArg::SharedObject{
        id: policy_object.object_id,
        initial_shared_version: policy_object.version,
        mutable: true,
    });
    ptb.input(policy_arg).unwrap();

    // policy_cap owned 0x2::transfer_policy::TransferPolicyCap<0xbe96c8adaab4785c4ff5e383cacdef0e74e7b72cac5773c234e84c21298029b1::bassinet_nft::BassinetNFT>
    let policy_cap_type = "0x2::transfer_policy::TransferPolicyCap<".to_owned() + &config.package_id + "::bassinet_nft::BassinetNFT>";
    let policy_cap = get_owned_object(policy_cap_type, sender, ObjectID::from_hex_literal("0x2").unwrap(), "transfer_policy".to_owned()).await;
    if policy_cap.is_err() {
        return Err(anyhow!(policy_cap.err().unwrap().to_string()))
    }
    let policy_cap_object = policy_cap.unwrap();
    let policy_cap_arg = CallArg::Object(ObjectArg::ImmOrOwnedObject(policy_cap_object.object_ref()));
    ptb.input(policy_cap_arg).unwrap();

    // app_name
    let app_name = "Bassinet";
    let app_name_arg = CallArg::Pure(bcs::to_bytes(&app_name).unwrap());
    ptb.input(app_name_arg).unwrap();

    let collection_id = nft_config.collection_id.clone();

    // 调取Api获取Collection信息
    // description
    let description = nft_config.description.clone();
    let description_arg = CallArg::Pure(bcs::to_bytes(&description).unwrap());
    ptb.input(description_arg).unwrap();
    
    // collection_id
    let collection_id_arg = CallArg::Pure(bcs::to_bytes(&collection_id).unwrap());
    ptb.input(collection_id_arg).unwrap();

    // collection_url
    let collection_url = nft_config.collection_url.clone();
    let collection_url_arg = CallArg::Pure(bcs::to_bytes(&collection_url).unwrap());
    ptb.input(collection_url_arg).unwrap();

    // limit
    let limit = nft_config.limit;
    let limit_arg = CallArg::Pure(bcs::to_bytes(&limit).unwrap());
    ptb.input(limit_arg).unwrap();

    // rewards_quantity
    let rewards_quantity = nft_config.rewards_quantity;
    let rewards_quantity_arg = CallArg::Pure(bcs::to_bytes(&rewards_quantity).unwrap());
    ptb.input(rewards_quantity_arg).unwrap();

    // minting_price
    let minting_price = nft_config.minting_price;
    let minting_price_arg = CallArg::Pure(bcs::to_bytes(&minting_price).unwrap());
    ptb.input(minting_price_arg).unwrap();

    ptb.command(Command::move_call(package, module, function, 
        vec![],
         vec![Argument::Input(0), Argument::Input(1), Argument::Input(2), Argument::Input(3), Argument::Input(4), Argument::Input(5), Argument::Input(6), Argument::Input(7), Argument::Input(8), Argument::Input(9), Argument::Input(10)]));

    let builder = ptb.finish();
    let gas_budget = 1_000_000_000;
    let gas_price = sui_test.read_api().get_reference_gas_price().await?;

    // create the transaction data that will be sent to the network
    let tx_data = TransactionData::new_programmable(
        sender,
        vec![gas_coin.unwrap().object_ref()],
        builder,
        gas_budget,
        gas_price,
    );

    // 4) sign transaction
    let keystore = FileBasedKeystore::new(&PathBuf::from_str(key_store_path).unwrap())?;
    let signature = keystore.sign_secure(&sender, &tx_data, Intent::sui_transaction())?;

    // 5) execute the transaction
    print!("Executing the transaction...");
    let transaction_response = sui_test
        .quorum_driver_api()
        .execute_transaction_block(
            Transaction::from_data(tx_data, vec![signature]),
            SuiTransactionBlockResponseOptions::full_content(),
            Some(ExecuteTransactionRequestType::WaitForLocalExecution),
        )
        .await?;
    let status = transaction_response.status_ok();
    if status.is_some() && status.unwrap() == true {
        println!("{}", transaction_response);
    }else {
        let message = format!("{}", transaction_response.effects.unwrap().into_status());
        return Err(anyhow!(message))
    }
    Ok(())
}

/// 获取指定类型的Object
pub async fn get_owned_object(object_type: String, address: SuiAddress, package_id: ObjectID, module: String) -> Result<SuiObjectData, anyhow::Error> {
    let sui_test = SuiClientBuilder::default()
        // .ws_url("wss://sui-testnet-rpc.publicnode.com")
        // .build("https://sui-testnet-rpc.publicnode.com")
        .build("https://fullnode.testnet.sui.io:443")
        .await?;

    let module_filter = SuiObjectDataFilter::MoveModule { package: package_id, module: Identifier::new(module).map_err(|e| anyhow!(e))?};
    let tag = parse_sui_struct_tag(object_type.as_str()).unwrap();
    let tag_filter = SuiObjectDataFilter::StructType(tag);
    let address_filter = SuiObjectDataFilter::AddressOwner(address);
    let mut filters: Vec<SuiObjectDataFilter> = Vec::new();
    filters.push(module_filter);
    filters.push(tag_filter);
    filters.push(address_filter);
    
    let filter_argument = SuiObjectDataFilter::MatchAll(filters);
    let query = SuiObjectResponseQuery::new_with_filter(filter_argument);

    let coins = sui_test.read_api()
    .get_owned_objects(address, Some(query), None, Some(1))
    .await?;
    // println!("{:?}", coins);
    if coins.data.len() < 1 {
        return Err(anyhow!("No Object:{}", &object_type))
    }
    let object = coins.data.into_iter().next().unwrap();
    return Ok(object.data.unwrap())
}

/// 获取指定ID的Object Share/Immutable
pub async fn get_object(object_id: ObjectID) -> Result<SuiObjectData, anyhow::Error> {
    let sui_test = SuiClientBuilder::default()
    // .ws_url("wss://sui-testnet-rpc.publicnode.com")
    // .build("https://sui-testnet-rpc.publicnode.com")
    .build("https://fullnode.testnet.sui.io:443")
    .await?;

    let response = sui_test.read_api()
    .get_object_with_options(
        object_id, 
        SuiObjectDataOptions {
        show_type: true,
        show_owner: true,
        show_previous_transaction: false,
        show_display: false,
        show_content: false,
        show_bcs: false,
        show_storage_rebate: false, }
    ).await;
    
    if response.is_err() {
        return Err(anyhow!(response.err().unwrap().to_string()))
    }

    let data = response.unwrap().data;
    if data.is_none() {
        return Err(anyhow!("No Object,ObjectID={}", object_id))
    }
    Ok(data.unwrap())
}