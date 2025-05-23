
pub async fn get_coin() {
    let sui = SuiClientBuilder::default()
        // .ws_url("wss://sui-testnet-rpc.publicnode.com")
        // .build("https://sui-testnet-rpc.publicnode.com")
        .build("https://fullnode.testnet.sui.io:443")
        .await?;

    let activa_address = SuiAddress::from_bytes(hex::decode("87e487cd6b1c7a53f91999eb3a5372ced201b614b26924ba4cc1d282a2240c07").unwrap()).unwrap();

    // Get coins for this address. Coins can be filtered by `coin_type`
    // (e.g., 0x168da5bf1f48dafc111b0a488fa454aca95e0b5e::usdc::USDC) or
    // use `None` for the default `Coin<SUI>` which is represented as
    // "0x2::sui::SUI"
    let coin_type = Some("0x2::sui::SUI".to_string());
    let coins = sui
        .coin_read_api()
        .get_coins(active_address, coin_type.clone(), None, Some(5)) // get the first five coins
        .await?;
    println!(" *** Coins ***");
    println!("{:?}", coins);
    println!(" *** Coins ***\n");

    // Get all coins
    // This function works very similar to the get_coins function, except it does not take
    // a coin_type filter argument and it returns all coin types associated with this address
    let all_coins = sui
        .coin_read_api()
        .get_all_coins(active_address, None, Some(5)) // get the first five coins
        .await?;
    println!(" *** All coins ***");
    println!("{:?}", all_coins);
    println!(" *** All coins ***\n");

    // Get coins as a stream
    // Similar to the previous functions, except it returns the coins as a stream.
    let coins_stream = sui.coin_read_api().get_coins_stream(active_address, None);

    println!(" *** Coins Stream ***");
    coins_stream
        .for_each(|coin| {
            println!("{:?}", coin);
            future::ready(())
        })
        .await;
    println!(" *** Coins Stream ***\n");

    // Select coins based on the provided coin type (SUI in this example). Use `None` for the default Sui coin
    let select_coins = sui
        .coin_read_api()
        .select_coins(active_address, coin_type, 1, vec![])
        .await?;

    println!(" *** Select Coins ***");
    println!("{:?}", select_coins);
    println!(" *** Select Coins ***\n");

    // Balance
    // Returns the balance for the specified coin type for this address,
    // or if None is passed, it will use Coin<SUI> as the coin type
    let balance = sui
        .coin_read_api()
        .get_balance(active_address, None)
        .await?;

    // Total balance
    // Returns the balance for each coin owned by this address
    let total_balance = sui.coin_read_api().get_all_balances(active_address).await?;
    println!(" *** Balance + Total Balance *** ");
    println!("Balance: {:?}", balance);
    println!("Total Balance: {:?}", total_balance);
    println!(" *** Balance + Total Balance ***\n ");

    // Return the coin metadata for the Coin<SUI>
    let coin_metadata = sui
        .coin_read_api()
        .get_coin_metadata("0x2::sui::SUI".to_string())
        .await?;

    println!(" *** Coin Metadata *** ");
    println!("{:?}", coin_metadata);
    println!(" *** Coin Metadata ***\n ");

    // Total Supply
    let total_supply = sui
        .coin_read_api()
        .get_total_supply("0x2::sui::SUI".to_string())
        .await?;
    println!(" *** Total Supply *** ");
    println!("{:?}", total_supply);
    println!(" *** Total Supply ***\n ");

    // ************ END OF COIN READ API ************ //
    Ok(())
}