use std::{fs, path::PathBuf, process::{Command, Stdio}};

use anyhow::{anyhow};
use fastcrypto::encoding::{Base64, Encoding};
use serde_json::{Value};
use sui_sdk::types::base_types::ObjectID;

use crate::{archive::unpack_bassinet, sui_service::publish_nft, template::bassinet_nft::{bassinet_nft_move_publish_template, bassinet_nft_move_template}};

use super::{init_config_nft, NftPublishedResult};

#[derive(Debug)]
pub struct NftServiceConfig {
    pub account: String,
    pub wallet_address: String,
    pub dir: PathBuf,
    pub collection_id: String,
    // pub limit: u64,
    // pub rewards_quantity: u64,
    // pub minting_price: u64,
    pub creator: String,
    pub provider: String,
    pub coin_package_id: String,
    pub package_id: String
}

#[derive(Debug)]
pub struct NftConfigInfo {
    pub description: String,
    pub collection_id: String,
    pub collection_url: String,
    pub limit: u64,
    pub rewards_quantity: u64,
    pub minting_price: u64
}

impl NftServiceConfig {

    pub fn new(account: String, wallet_address: String, dir: PathBuf, collection_id: String, /*limit: u64, rewards_quantity: u64, minting_price: u64, */creator: String, provider: String, coin_package_id: String, package_id: String) -> Self {
        Self{
            account,
            wallet_address,
            dir,
            collection_id,
            // limit,
            // rewards_quantity,
            // minting_price,
            creator,
            provider,
            coin_package_id,
            package_id
        }
    }

    /// 发行NFT
    pub  async fn launch(&mut self, key_store_path: &str) -> Result<NftPublishedResult, anyhow::Error> {
        // 设置当前环境(测试环境)
        let change_env_child = Command::new("sui").arg("client").arg("switch").arg("--env").arg("testnet")
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
        let change_env_output = change_env_child.wait_with_output().unwrap();
        if !change_env_output.status.success() {
            let stderr = String::from_utf8(change_env_output.stderr).unwrap();
            return Err(anyhow!(stderr));
        }

        // 创建合约目录
        let base_dir = self.wallet_address.clone();
        // 创建合约目录
        let dir = self.dir.join(base_dir.strip_prefix("0x").unwrap_or(base_dir.as_str()));
        if !dir.exists(){
            return Err(anyhow!("该账户合约目录不存在"))
        }
        let nft_dir = dir.join(&self.collection_id);
        if nft_dir.exists() {
            return Err(anyhow!("合约目录:{}已存在", self.collection_id));
        }
        let create_dir_result = fs::create_dir(&nft_dir);
        if create_dir_result.is_err() {
            return Err(anyhow!(create_dir_result.err().unwrap().to_string()))
        }
    
        // 复制代码
        let copy_result= unpack_bassinet(&nft_dir);
        if copy_result.is_err() {
            return Err(anyhow!(copy_result.err().unwrap().to_string()))
        }
        let path = nft_dir.join("Move.toml");
        let template_result = bassinet_nft_move_template(&path, self);
        if template_result.is_err() {
            return Err(anyhow!(template_result.err().unwrap().to_string()))
        }

        // 编译代码
        let child = Command::new("sui").current_dir(&nft_dir).arg("move").arg("build").arg("--dump-bytecode-as-base64")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    
        let output = child.wait_with_output().unwrap();
        if !output.status.success() {
            let stderr = String::from_utf8(output.stderr).unwrap();
            return Err(anyhow!(stderr));
        }
        // 编译合约
        let base64_code = String::from_utf8(output.stdout).unwrap();
        let value : Value = serde_json::from_str(base64_code.as_str()).unwrap();
        let modules_arr = value["modules"].as_array().unwrap();
        let mut modules: Vec<Vec<u8>> = Vec::new();
        for index in 0..modules_arr.len() {
            modules.push(Base64::decode(modules_arr[index].as_str().unwrap()).unwrap());
            // modules.push(modules_arr[index].as_str().unwrap().as_bytes().to_vec());
        }
        let mut object_ids = Vec::new();
        let arr = value["dependencies"].as_array().unwrap();
        for index in 0..arr.len() {
            object_ids.push(ObjectID::from_hex_literal(arr[index].as_str().unwrap()).map_err(|e| anyhow!(e))?);
        }

        // 发布NFT合约
        let publish_result = publish_nft(self, modules.clone(), object_ids, key_store_path).await;
        if publish_result.is_err() {
            return Err(anyhow!(publish_result.err().unwrap().to_string()))
        }
        let result = publish_result.unwrap();
        self.package_id = result.package_id.clone();
        let move_publish_result = bassinet_nft_move_publish_template(&nft_dir.join("Move.toml"), self);
        if move_publish_result.is_err() {
            return Err(anyhow!(move_publish_result.err().unwrap().to_string()))
        }
        Ok(result)
    }

    /// 初始化配置
    pub async fn init_config(&self, nft_config: &NftConfigInfo, policy_id: ObjectID, mint_id: ObjectID, key_store_path: &str) -> Result<(), anyhow::Error> {
        init_config_nft(&self, nft_config, policy_id, mint_id, key_store_path).await?;
        Ok(())
    }
}