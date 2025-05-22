/*
/// Module: bassinet_coin
module bassinet_coin::bassinet_coin;
*/

// For Move coding conventions, see
// https://docs.sui.io/concepts/sui-move-concepts/conventions

module bassinet_coin::bassinet_coin;

use sui::coin::{Self, Coin};
use sui::balance::{Self, Balance};
use sui::dynamic_field as df;
use std::type_name::{Self};
use std::string::{String};

// 10亿10^6
const MAX_SUPPLY: u64 = 1_000_000_000_000_000;

/// For when there's no profits to claim.
const ENoProfits: u64 = 0;
const ENotAuthorized: u64 = 1;

public struct TreasuryLock has key, store {
    id: UID,
    // 最大供应量
    max_supply: u64,
    // 总供应量
    total_supply: u64,
    // 筹造计数
    minting_counter: u64,
    // 总共激励数量
    total_rewards: Balance<BASSINET_COIN>,
    // 创作者
    creator: address,
    // 创作者激励
    creator_rewards: Balance<BASSINET_COIN>,
    // 平台方
    platform_provider: address,
    // 平台方激励
    platform_provider_rewards: Balance<BASSINET_COIN>,
}

/// admin cap
public struct AdminCap has key, store {
    id: UID
}

/// Custom key under which the app cap is attached.
public struct AppKey<phantom T> has copy, store, drop {}

/// Capability granting mint permission.
public struct MintAppCap<phantom T> has store, drop {
    app_name: String,
    app_type: std::ascii::String,
    /// 激励数量
    rewards_quantity: u64,
    /// 激励次数限制
    minting_limit: u64,
    /// 激励计数
    minting_counter: u64,
    /// 总激励
    total_rewards: u64
}

public struct BASSINET_COIN has drop {}

/// 初始调用
fun init(witness: BASSINET_COIN, ctx: &mut TxContext) {
    let (mut treasury_cap, metadata) = coin::create_currency(
        witness,
        6,
        b"BASSINET_COIN",
        b"",
        b"",
        option::none(),
        ctx,
    );
    // Freezing this object makes the metadata immutable, including the title, name, and icon image.
    // If you want to allow mutability, share it with public_share_object instead.
    transfer::public_freeze_object(metadata);

    let creator = @creator;
    let platform_provider = @platform_provider;

    // 发行10亿
    let minting_coin = coin::mint(&mut treasury_cap, MAX_SUPPLY, ctx);
    transfer::public_freeze_object(treasury_cap);

    let admin_cap = AdminCap{
        id: object::new(ctx)
    };
    transfer::public_transfer(admin_cap, ctx.sender());

    let lock = TreasuryLock {
        id: object::new(ctx),
        max_supply: MAX_SUPPLY,
        total_supply: 0,
        minting_counter: 0,
        total_rewards: minting_coin.into_balance(),
        creator: creator,
        creator_rewards: balance::zero<BASSINET_COIN>(),
        platform_provider: platform_provider,
        platform_provider_rewards: balance::zero<BASSINET_COIN>()
    };
    transfer::public_share_object(lock)
}

/// 挖掘激励
public fun mint<T>(app: &mut UID, self: &mut TreasuryLock, ctx: &mut TxContext): Coin<BASSINET_COIN>{
    assert!(is_authorized<T>(app), ENotAuthorized);
    let app_cap = app_cap_mut<T>(app);

    // 已经达到最大限制
    if (app_cap.minting_counter  == app_cap.minting_limit) {
        return coin::zero<BASSINET_COIN>(ctx)
    };

    let total_rewards = self.total_rewards.value();
    // 无剩余激励
    if (total_rewards == 0) {
        return coin::zero<BASSINET_COIN>(ctx)
    };

    // 激励数量
    let rewards = if (total_rewards > app_cap.rewards_quantity) {
        app_cap.rewards_quantity
    }else {
        total_rewards
    };
    
    // plan
    // minter 49%
    // creator 30%
    // platform provider 21%
    let recipient_amount = ((((rewards as u128) * (49 as u128)) / 100) as u64);
    let creator_amount = ((((rewards as u128) * (30 as u128)) / 100) as u64);
    let provider_amount = rewards - recipient_amount - creator_amount;

    // mint to recipient
    let recipient_balance = balance::split(&mut self.total_rewards, recipient_amount);

    // mint to creator
    let creator_balance = balance::split(&mut self.total_rewards, creator_amount);
    balance::join(&mut self.creator_rewards, creator_balance);

    // mint to platform provider
    let provider_balance = balance::split(&mut self.total_rewards, provider_amount);
    balance::join(&mut self.platform_provider_rewards, provider_balance);
    
    self.minting_counter = self.minting_counter + 1;
    self.total_supply= self.total_supply + rewards;

    app_cap.minting_counter = app_cap.minting_counter + 1;
    app_cap.total_rewards = app_cap.total_rewards + rewards;

    coin::from_balance(recipient_balance, ctx)
}

// === Authorization ===

/// Attach an `MintAppCap` under an `AppKey` to grant an application access
/// to minting and burning.
public fun authorize_app<T>(
    _: &AdminCap,
    app: &mut UID,
    app_name: String,
    // 激励数量
    rewards_quantity: u64,
    // 激励次数限制
    minting_limit: u64
) {
    df::add(app, AppKey<T> {},
        MintAppCap<T> {
            app_name: app_name,
            app_type: type_name::into_string(type_name::get<T>()),
            // 激励数量
            rewards_quantity: rewards_quantity,
            // 激励次数限制
            minting_limit: minting_limit,
            // 激励计数
            minting_counter: 0,
            total_rewards: 0
        }
    )
}

/// Detach the `MintAppCap` from the application to revoke access.
public fun revoke_auth<T>(_: &AdminCap, app: &mut UID) {
    let MintAppCap<T> {
        app_name: _,
        app_type:_,
        rewards_quantity: _,
        minting_limit: _,
        minting_counter: _,
        total_rewards: _
    } = df::remove(app, AppKey<T> {});
}

/// Check whether an Application has a permission to mint or
/// burn a specific NFT.
public fun is_authorized<T>(app: &UID): bool {
    df::exists_<AppKey<T>>(app, AppKey<T> {})
}

/// Returns the `MintAppCap`
fun app_cap_mut<T>(app: &mut UID): &mut MintAppCap<T> {
    df::borrow_mut<AppKey<T>, MintAppCap<T>>(app, AppKey<T> {})
}

// === Profits ===

/// 领取激励
public entry fun take_rewards(self: &mut TreasuryLock, recipient: address, ctx: &mut TxContext) {
    let rewards = take_rewards_(self, ctx);
    if (rewards.value() > 0) {
        transfer::public_transfer(rewards, recipient);
    }else {
        rewards.destroy_zero();
    };
}

/// 领取激励
public fun take_rewards_(self: &mut TreasuryLock, ctx: &mut TxContext): Coin<BASSINET_COIN> {
    let sender = ctx.sender();
    if (is_creator(self, sender)) {
        return take_creator_profits(self, ctx)
    }else if (is_platform_provider(self, sender)) {
        return take_provider_profits(self, ctx)
    };
    coin::zero<BASSINET_COIN>(ctx)
}

/// 领取创作者激励
fun take_creator_profits(self: &mut TreasuryLock, ctx: &mut TxContext): Coin<BASSINET_COIN> {
    let amount = balance::value(&self.creator_rewards);
    assert!(amount > 0, ENoProfits);
    // Take a transferable `Coin` from a `Balance`
    coin::take(&mut self.creator_rewards, amount, ctx)
}

/// 领取平台激励
fun take_provider_profits(self: &mut TreasuryLock, ctx: &mut TxContext): Coin<BASSINET_COIN> {
    let amount = balance::value(&self.creator_rewards);
    assert!(amount > 0, ENoProfits);
    // Take a transferable `Coin` from a `Balance`
    coin::take(&mut self.creator_rewards, amount, ctx)
}

/// 是否平台方
fun is_platform_provider(self: &TreasuryLock, operator_address: address): bool {
    self.platform_provider == operator_address
}

/// 是否创作者
fun is_creator(self: &TreasuryLock, operator_address: address): bool {
    self.creator == operator_address
}