use std::{env, sync::Arc, time::Duration};

use amqprs::{callbacks::{DefaultChannelCallback, DefaultConnectionCallback}, channel::{ExchangeDeclareArguments, ExchangeDeleteArguments}, connection::{Connection, OpenConnectionArguments}};
use anyhow::Context;
use tokio::time::sleep;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error>{
    dotenvy::dotenv()?;

    // tracing_subscriber::fmt::init();
    tracing_subscriber::registry()
    .with(
        tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
    ).with(tracing_subscriber::fmt::layer())
    .init();

    let config = Arc::new(load_config().await);
    // // println!("config:{:?}", config.clone());
    // // 删除exchange
    // let _ = delete_exchange(config.clone()).await?;
    // 声明exchange
    let _ = declare_exchange(config.clone()).await?;
    Ok(())
}

/// Application configuration data.
#[derive(Debug)]
pub struct Config {
    pub virtual_host: String,
    pub host: String,
    pub password: String,
    pub port: u16,
    pub username: String,
}

pub async fn declare_exchange(cfg: Arc<Config>) -> anyhow::Result<()> {
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

    let exchange_name = "bassinet.topic";
    channel
        .exchange_declare(ExchangeDeclareArguments::new(exchange_name, "topic").durable(true).finish())
        .await
        .context("declare exchange failed")?;
    // explicitly close
    channel.close().await.unwrap();
    connection.close().await.unwrap();

    Ok(())
}

pub async fn delete_exchange(cfg: Arc<Config>) -> anyhow::Result<()> {
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

    let exchange_name = "bassinet.topic";
    channel
        .exchange_delete(ExchangeDeleteArguments::new(exchange_name))
        .await
        .context("delete exchange failed")?;
    // explicitly close
    channel.close().await.unwrap();
    connection.close().await.unwrap();

    Ok(())
}

pub async fn load_config() -> Config {
    sleep(Duration::from_millis(10)).await; // delay to simulate loading configuration
    Config {
        virtual_host: env::var("RABBIT_VHOST").unwrap_or("/".to_owned()),
        host: env::var("RABBIT_HOST").unwrap_or("localhost".to_owned()),
        password: env::var("RABBIT_PASSWORD").unwrap_or("guest".to_owned()),
        port: env::var("RABBIT_PORT")
            .map(|s| s.parse::<u16>().expect("can't parse RABBIT_PORT"))
            .unwrap_or(5672),
        username: env::var("RABBIT_USER").unwrap_or("guest".to_owned()),
    }
}