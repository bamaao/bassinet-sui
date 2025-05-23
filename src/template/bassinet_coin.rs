use std::{collections::HashMap, fs::{self}, path::{Path, PathBuf}};

use anyhow::anyhow;
use text_placeholder::Template;

use crate::sui_service::digital_service::OpenDigitalServiceConfig;

/// bassinet_coin/sources/bassinet_coin.move tempalte
pub fn bassinet_coin_template(dest_path: &PathBuf, config: &OpenDigitalServiceConfig) -> Result<(), anyhow::Error> {
    let template_path = std::env::var("BASSINET_TEMPLATE_PATH").expect("BASSINET_TEMPLATE_PATH must be set");
    let template_path = Path::new(&template_path);
    let path = template_path.join("bassinet_coin_template");
    let template_content = fs::read_to_string(&path)?;
    let template = Template::new(&template_content);

    let mut table = HashMap::new();
    table.insert("symbol", config.symbol.as_str());
    table.insert("name", config.name.as_str());
    table.insert("description", config.description.as_str());
    table.insert("icon_url", config.icon_url.as_str());
    
    let content = template.fill_with_hashmap(&table);
    let result = fs::write(dest_path, content);
    if result.is_err() {
        return Err(anyhow!(result.err().unwrap().to_string()))
    }
    Ok(())
}

/// bassinet_coin/Move.toml template
pub fn bassinet_coin_move_template(dest_path: &PathBuf, config: &OpenDigitalServiceConfig) -> Result<(), anyhow::Error> {
    let template_path = std::env::var("BASSINET_TEMPLATE_PATH").expect("BASSINET_TEMPLATE_PATH must be set");
    let template_path = Path::new(&template_path);
    let path = template_path.join("bassinet_coin_move_template");
    let template_content = fs::read_to_string(path)?;
    let template = Template::new(&template_content);

    let mut table = HashMap::new();
    table.insert("package_id", config.package_id.as_str());
    table.insert("creator", config.creator.as_str());
    table.insert("provider", config.provider.as_str());
    
    let content = template.fill_with_hashmap(&table);
    let result = fs::write(dest_path, content);
    if result.is_err() {
        return Err(anyhow!(result.err().unwrap().to_string()))
    }
    Ok(())
}

/// bassinet_coin/Move.toml template
pub fn bassinet_coin_move_publish_template(dest_path: &PathBuf, config: &OpenDigitalServiceConfig) -> Result<(), anyhow::Error> {
    let template_path = std::env::var("BASSINET_TEMPLATE_PATH").expect("BASSINET_TEMPLATE_PATH must be set");
    let template_path = Path::new(&template_path);
    let path = template_path.join("bassinet_coin_move_publish_template");
    let template_content = fs::read_to_string(path)?;
    let template = Template::new(&template_content);

    let mut table = HashMap::new();
    table.insert("package_id", config.package_id.as_str());
    table.insert("creator", config.creator.as_str());
    table.insert("provider", config.provider.as_str());
    
    let content = template.fill_with_hashmap(&table);
    let result = fs::write(dest_path, content);
    if result.is_err() {
        return Err(anyhow!(result.err().unwrap().to_string()))
    }
    Ok(())
}