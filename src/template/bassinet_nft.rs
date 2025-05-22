use std::{collections::HashMap, fs::{self}, path::PathBuf};

use anyhow::anyhow;
use text_placeholder::Template;

use crate::sui_service::nft_service::NftServiceConfig;

/// bassinet/Move.toml template
pub fn bassinet_nft_move_template(dest_path: &PathBuf, config: &NftServiceConfig) -> Result<(), anyhow::Error> {
    let template_content = fs::read_to_string("templates/bassinet_nft_move_template")?;
    let template = Template::new(&template_content);

    let mut table = HashMap::new();
    table.insert("package_id", config.package_id.as_str());
    table.insert("creator", config.creator.as_str());
    table.insert("provider", config.provider.as_str());
    table.insert("bassinet_coin", config.coin_package_id.as_str());
    
    let content = template.fill_with_hashmap(&table);
    let result = fs::write(dest_path, content);
    if result.is_err() {
        return Err(anyhow!(result.err().unwrap().to_string()))
    }
    Ok(())
}

/// bassinet/Move.toml template
pub fn bassinet_nft_move_publish_template(dest_path: &PathBuf, config: &NftServiceConfig) -> Result<(), anyhow::Error> {
    let template_content = fs::read_to_string("templates/bassinet_nft_move_publish_template")?;
    let template = Template::new(&template_content);

    let mut table = HashMap::new();
    table.insert("package_id", config.package_id.as_str());
    table.insert("creator", config.creator.as_str());
    table.insert("provider", config.provider.as_str());
    table.insert("bassinet_coin", config.coin_package_id.as_str());
    
    let content = template.fill_with_hashmap(&table);
    let result = fs::write(dest_path, content);
    if result.is_err() {
        return Err(anyhow!(result.err().unwrap().to_string()))
    }
    Ok(())
}