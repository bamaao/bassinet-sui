use std::{path::PathBuf, str::FromStr};
use anyhow::anyhow;
use fastcrypto::{ed25519::{Ed25519KeyPair, Ed25519PublicKey}, traits::{KeyPair, Signer, ToFromBytes}};
use rand::thread_rng;
use shared_crypto::intent::Intent;
use sui_keys::keystore::{AccountKeystore, FileBasedKeystore};
use sui_sdk::{rpc_types::{Coin, SuiTransactionBlockResponseOptions}, types::{base_types::{ObjectID, SuiAddress}, programmable_transaction_builder::ProgrammableTransactionBuilder, quorum_driver_types::ExecuteTransactionRequestType, transaction::{Argument, CallArg, Command, ObjectArg, Transaction, TransactionData}, Identifier}, SuiClientBuilder};

const PACKAGE_ID_CONST: &str = "0xbf9c318ab31871ff47adffadc78dd1dfe5c65d7bcad492645e1c6cc94c9f9f3e";

/// 绑定钱包
pub async fn binding_account() -> Result<(), anyhow::Error> {
    let sui_test = SuiClientBuilder::default()
        // .ws_url("wss://sui-testnet-rpc.publicnode.com")
        // .build("https://sui-testnet-rpc.publicnode.com")
        .build("https://fullnode.testnet.sui.io:443")
        .await?;
    
    let kp = Ed25519KeyPair::generate(&mut thread_rng());
    let message = uuid::Uuid::new_v4().to_string();
    println!("message:{}", hex::encode(message.as_bytes()));
    let sign = kp.sign(message.as_bytes());
    println!("sign:{}", hex::encode(sign.sig.to_bytes()));
    println!("verifying_key:{}", hex::encode(kp.public().as_bytes()));
    let private_key = kp.private();
    println!("private key:{}", hex::encode(private_key.as_bytes()));
    let public_key = Ed25519PublicKey::from(&private_key);
    let public_key_bytes = public_key.as_bytes();

    let mut ptb = ProgrammableTransactionBuilder::new();
    let sender = SuiAddress::from_bytes(hex::decode("87e487cd6b1c7a53f91999eb3a5372ced201b614b26924ba4cc1d282a2240c07").unwrap()).unwrap();

    // we need to find the coin we will use as gas
    let coins = sui_test
        .coin_read_api()
        .get_coins(sender, None, None, None)
        .await?;
    let coin = coins.data.into_iter().next().unwrap();

    // let pkg_id = "0xbf9c318ab31871ff47adffadc78dd1dfe5c65d7bcad492645e1c6cc94c9f9f3e";
    let package = ObjectID::from_hex_literal(PACKAGE_ID_CONST).map_err(|e| anyhow!(e))?;
    let module = Identifier::new("digital_service").map_err(|e| anyhow!(e))?;
    let function = Identifier::new("bind_account").map_err(|e| anyhow!(e))?;

    let open_fee_object_id: ObjectID = "0xbdab5af0ffdc7a4f359a3f49da3c2b5ab651ad7274f809b0c86ec1c17d735008".parse().unwrap();
    // let open_fee = ObjectArg::ImmOrOwnedObject((open_fee_object_id, 349179435.into(), ObjectDigest::from_str("BGGv1fjvVjkKNxTQWJivV28P4GhRDQ2sQqx2kB2YRJyq").unwrap()));
    let open_fee = ObjectArg::SharedObject{id: open_fee_object_id, initial_shared_version: 349179470.into(), mutable: true};
    let open_fee_arg = CallArg::Object(open_fee);
    let signature = CallArg::Pure(bcs::to_bytes(&hex::encode(sign.sig.to_bytes())).unwrap());
    let public_key = CallArg::Pure(bcs::to_bytes(&hex::encode(public_key_bytes)).unwrap());
    let message = CallArg::Pure(bcs::to_bytes(&hex::encode(message.as_bytes())).unwrap());

    ptb.input(open_fee_arg).unwrap();
    ptb.input(signature).unwrap();
    ptb.input(public_key).unwrap();
    ptb.input(message).unwrap();

    ptb.command(Command::move_call(package, module, function, vec![], vec![Argument::Input(0), Argument::Input(1), Argument::Input(2), Argument::Input(3)]));

    let builder = ptb.finish();
    let gas_budget = 500_000_000;
    let gas_price = sui_test.read_api().get_reference_gas_price().await?;

    // create the transaction data that will be sent to the network
    let tx_data = TransactionData::new_programmable(
        sender,
        vec![coin.object_ref()],
        builder,
        gas_budget,
        gas_price,
    );

    // 4) sign transaction
    let keystore = FileBasedKeystore::new(&PathBuf::from_str("D:/Users/zouyc/.sui/sui_config/sui.keystore").unwrap())?;
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
    Ok(())
}

/// 开通账户数字服务
pub async fn open_digital_service() -> Result<(), anyhow::Error> {
    let sui_test = SuiClientBuilder::default()
        // .ws_url("wss://sui-testnet-rpc.publicnode.com")
        // .build("https://sui-testnet-rpc.publicnode.com")
        .build("https://fullnode.testnet.sui.io:443")
        .await?;

    let mut ptb = ProgrammableTransactionBuilder::new();
    let sender = SuiAddress::from_bytes(hex::decode("87e487cd6b1c7a53f91999eb3a5372ced201b614b26924ba4cc1d282a2240c07").unwrap()).unwrap();

    // we need to find the coin we will use as gas
    let coins = sui_test
        .coin_read_api()
        .get_coins(sender, None, None, None)
        .await?;
    // let coin = coins.data.into_iter().next().unwrap();
    let mut gas_coin : Option<Coin> = Option::None;
    let mut iter = coins.data.into_iter();

    let mut payment: Option<Coin> = Option::None;
    let paid = 1_000_000_000u64;
    while let Some(coin) = iter.next() {
        if coin.balance >= paid {
            payment = Some(coin);
        }else {
            gas_coin = Some(coin);
        }
    }

    // let pkg_id = "0x037e99ab5623b5f1fccfcbadd460c30b8b3e4c858d85e94015e29b65e6f45ed8";
    let package = ObjectID::from_hex_literal(PACKAGE_ID_CONST).map_err(|e| anyhow!(e))?;
    let module = Identifier::new("digital_service").map_err(|e| anyhow!(e))?;
    let function = Identifier::new("open_digital_service").map_err(|e| anyhow!(e))?;

    let open_fee_object_id: ObjectID = "0xbdab5af0ffdc7a4f359a3f49da3c2b5ab651ad7274f809b0c86ec1c17d735008".parse().unwrap();
    // let open_fee = ObjectArg::ImmOrOwnedObject((open_fee_object_id, 349179435.into(), ObjectDigest::from_str("BGGv1fjvVjkKNxTQWJivV28P4GhRDQ2sQqx2kB2YRJyq").unwrap()));
    let open_fee = ObjectArg::SharedObject{id: open_fee_object_id, initial_shared_version: 349179470.into(), mutable: true};
    let open_fee_arg = CallArg::Object(open_fee);
    let payment_arg = CallArg::Object(ObjectArg::ImmOrOwnedObject(payment.unwrap().object_ref()));

    ptb.input(open_fee_arg).unwrap();
    ptb.input(payment_arg).unwrap();

    ptb.command(Command::move_call(package, module, function, vec![], vec![Argument::Input(0), Argument::Input(1)]));

    let builder = ptb.finish();
    let gas_budget = 500_000_000;
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
    let keystore = FileBasedKeystore::new(&PathBuf::from_str("D:/Users/zouyc/.sui/sui_config/sui.keystore").unwrap())?;
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
    Ok(())
}


/// 发行NFT
pub async fn launch_nft() -> Result<(), anyhow::Error> {
    let sui_test = SuiClientBuilder::default()
        // .ws_url("wss://sui-testnet-rpc.publicnode.com")
        // .build("https://sui-testnet-rpc.publicnode.com")
        .build("https://fullnode.testnet.sui.io:443")
        .await?;

    let mut ptb = ProgrammableTransactionBuilder::new();
    let sender = SuiAddress::from_bytes(hex::decode("87e487cd6b1c7a53f91999eb3a5372ced201b614b26924ba4cc1d282a2240c07").unwrap()).unwrap();

    // we need to find the coin we will use as gas
    let coins = sui_test
        .coin_read_api()
        .get_coins(sender, None, None, None)
        .await?;
    // let coin = coins.data.into_iter().next().unwrap();
    let mut gas_coin : Option<Coin> = Option::None;
    let mut iter = coins.data.into_iter();

    let mut payment: Option<Coin> = Option::None;
    let paid = 1_000_000_000u64;
    while let Some(coin) = iter.next() {
        if coin.balance >= paid {
            payment = Some(coin);
        }else {
            gas_coin = Some(coin);
        }
    }

    // let pkg_id = "0x037e99ab5623b5f1fccfcbadd460c30b8b3e4c858d85e94015e29b65e6f45ed8";
    let package = ObjectID::from_hex_literal(PACKAGE_ID_CONST).map_err(|e| anyhow!(e))?;
    let module = Identifier::new("launch_service").map_err(|e| anyhow!(e))?;
    let function = Identifier::new("launch").map_err(|e| anyhow!(e))?;

    let launch_fee_object_id = "0x71e90627ff9d7aa9b50aec76447a09fd966d96e366451dbbabf042725d4d1ed8".parse().unwrap();
    let launch_fee = ObjectArg::SharedObject { id: launch_fee_object_id, initial_shared_version: 349179470.into(), mutable: true };
    let launch_fee_arg = CallArg::Object(launch_fee);
    let open_fee_object_id: ObjectID = "0xbdab5af0ffdc7a4f359a3f49da3c2b5ab651ad7274f809b0c86ec1c17d735008".parse().unwrap();
    // let open_fee = ObjectArg::ImmOrOwnedObject((open_fee_object_id, 349179435.into(), ObjectDigest::from_str("BGGv1fjvVjkKNxTQWJivV28P4GhRDQ2sQqx2kB2YRJyq").unwrap()));
    let open_fee = ObjectArg::SharedObject{id: open_fee_object_id, initial_shared_version: 349179470.into(), mutable: false};
    let open_fee_arg = CallArg::Object(open_fee);
    let collection_id = uuid::Uuid::new_v4().to_string();
    let collection_id_arg = CallArg::Pure(bcs::to_bytes(&collection_id).unwrap());
    let limit = Option::Some(10000u64);
    let limit_arg = CallArg::Pure(bcs::to_bytes(&limit).unwrap());
    let rewards_quantity = 100u64;
    let rewards_quantity_arg = CallArg::Pure(bcs::to_bytes(&rewards_quantity).unwrap());
    let minting_price = 1_000_000_000u64;
    let minting_price_arg = CallArg::Pure(bcs::to_bytes(&minting_price).unwrap());
    let payment_arg = CallArg::Object(ObjectArg::ImmOrOwnedObject(payment.unwrap().object_ref()));

    ptb.input(launch_fee_arg).unwrap();
    ptb.input(open_fee_arg).unwrap();
    ptb.input(collection_id_arg).unwrap();
    ptb.input(limit_arg).unwrap();
    ptb.input(rewards_quantity_arg).unwrap();
    ptb.input(minting_price_arg).unwrap();
    ptb.input(payment_arg).unwrap();

    ptb.command(Command::move_call(package, module, function, vec![], vec![Argument::Input(0), Argument::Input(1), Argument::Input(2), Argument::Input(3), Argument::Input(4), Argument::Input(5), Argument::Input(6)]));

    let builder = ptb.finish();
    let gas_budget = 500_000_000;
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
    let keystore = FileBasedKeystore::new(&PathBuf::from_str("D:/Users/zouyc/.sui/sui_config/sui.keystore").unwrap())?;
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
    Ok(())
}