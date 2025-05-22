use std::{fs, path::PathBuf, process::{Command, Stdio}};

use anyhow::{anyhow};
use fastcrypto::encoding::{Base64, Encoding};
use serde_json::{Value};
use sui_sdk::types::base_types::ObjectID;

use crate::{archive::unpack, sui_service::publish, template::bassinet_coin::{bassinet_coin_move_publish_template, bassinet_coin_move_template, bassinet_coin_template}};

use super::BassinetCoinPublishedResult;

#[derive(Debug)]
pub struct OpenDigitalServiceConfig {
    pub account: String,
    pub wallet_address: String,
    pub dir: PathBuf,
    pub symbol: String,
    pub name: String,
    pub description: String,
    pub icon_url: String,
    pub creator: String,
    pub provider: String,
    pub package_id: String
}

impl OpenDigitalServiceConfig {

    pub fn new(account: String, wallet_address: String, dir: PathBuf, symbol: String, name: String, description: String, icon_url: String, creator: String, provider: String, package_id: String) -> Self {
        Self{
            account,
            wallet_address,
            dir,
            symbol,
            name,
            description,
            icon_url,
            creator,
            provider,
            package_id
        }
    }

    /// 开通
    pub  async fn open(&mut self, key_store_path: &str) -> Result<BassinetCoinPublishedResult, anyhow::Error> {
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

        let base_dir = self.wallet_address.clone();
        // 创建合约目录
        let dir = self.dir.join(base_dir.strip_prefix("0x").unwrap_or(base_dir.as_str()));
        if dir.exists(){
            if !dir.is_dir() {
                return Err(anyhow!("无法创建合约目录,同名文件已存在"))
            }
        }else {
            let create_dir_result = fs::create_dir(&dir);
            if create_dir_result.is_err() {
                return Err(anyhow!(create_dir_result.err().unwrap().to_string()))
            }
        }

        let coin_dir = dir.join("bassinet_coin");
        if coin_dir.exists() {
            return Err(anyhow!("Bassinet Coin合约目录已存在"))
        }
    
        // 复制代码
        let copy_result= unpack(&dir);
        if copy_result.is_err() {
            return Err(anyhow!(copy_result.err().unwrap().to_string()))
        }
        let path = dir.join("bassinet_coin").join("sources").join("bassinet_coin.move");
        // println!("path:{:?}", path.as_os_str());
        let template_result = bassinet_coin_template(&path, self);
        if template_result.is_err() {
            return Err(anyhow!(template_result.err().unwrap().to_string()))
        }
        let move_result = bassinet_coin_move_template(&dir.join("bassinet_coin").join("Move.toml"), self);
        if move_result.is_err() {
            return Err(anyhow!(move_result.err().unwrap().to_string()))
        }

        // 编译代码
        let child = Command::new("sui").current_dir(dir.join("bassinet_coin")).arg("move").arg("build").arg("--dump-bytecode-as-base64")
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

        // 发布合约
        let publish_result = publish(self, modules.clone(), object_ids, key_store_path).await;
        if publish_result.is_err() {
            return Err(anyhow!(publish_result.err().unwrap().to_string()))
        }
        let result = publish_result.unwrap();
        // 填充模板
        self.package_id = result.package_id.clone();
        let move_publish_result = bassinet_coin_move_publish_template(&dir.join("bassinet_coin").join("Move.toml"), self);
        if move_publish_result.is_err() {
            return Err(anyhow!(move_publish_result.err().unwrap().to_string()))
        }
        Ok(result)
    }
}
