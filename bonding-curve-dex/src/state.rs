use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Item, Map};

// Add constants for data structure limits
pub const MAX_ORDERS_PER_PRICE: usize = 100_000;
pub const MAX_TRADES_PER_USER: usize = 100;
pub const MAX_ACTIVE_ORDERS_PER_USER: usize = 50;
pub const PRUNE_THRESHOLD: u64 = 7 * 24 * 60 * 60; // 7 days in seconds

// Constants for bonding curve
pub const BASE_PRICE: u128 = 100; // 0.0001 Huahua

// Configuration
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub token_factory: Addr,
    pub fee_collector: Addr,
    pub quote_token_total_supply: u128,
    pub bonding_curve_supply: u128,
    pub lp_supply: u128,
    pub maker_fee: Decimal, // in basis points (1/10000)
    pub taker_fee: Decimal, // in basis points (1/10000)
    pub enabled: bool,
    pub secondary_amm_address: Addr,
    pub base_token_denom: String,
}

// Token information
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: Uint128,
    pub initial_price: Uint128,
    pub max_price_impact: Uint128, // To guard against massive buys and sells
    pub graduated: bool,
}

// Order book structure
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct OrderBook {
    pub pair_id: String,
    pub buy_orders: BTreeMap<u128, Vec<Order>>,
    pub sell_orders: BTreeMap<u128, Vec<Order>>,
}

// Order structure
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Order {
    pub id: u64,
    pub owner: Addr,
    pub pair_id: String,
    pub token_amount: Uint128,
    pub price: Uint128,
    pub timestamp: u64,
    pub status: OrderStatus,
    pub filled_amount: Uint128,
    pub remaining_amount: Uint128,
    pub order_type: OrderType,
    pub created_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub enum OrderStatus {
    Active,
    Filled,
    Cancelled,
    PartiallyFilled { filled_amount: Uint128 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub enum OrderType {
    Buy,
    Sell,
}

// Trade history
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Trade {
    pub id: u64,
    pub pair_id: String,
    pub buy_order_id: u64,
    pub sell_order_id: u64,
    pub buyer: Addr,
    pub seller: Addr,
    pub token_amount: Uint128,
    pub price: Uint128,
    pub timestamp: u64,
    pub total_price: Uint128,
    pub maker_fee_amount: Uint128,
    pub taker_fee_amount: Uint128,
}

// Pool information
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Pool {
    pub pair_id: String,
    pub curve_slope: Uint128,
    pub token_address: Addr,
    pub total_reserve_token: Uint128,
    pub token_sold: Uint128,
    pub total_volume: Uint128,
    pub total_trades: Uint128, // Track total trading volume
    pub total_fees_collected: Uint128,
    pub last_price: Uint128, // Last traded price
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct TokenPair {
    pub base_token: String,  // Native or CW20 token address
    pub quote_token: String, // Native or CW20 token address
    pub base_decimals: u8,
    pub quote_decimals: u8,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PriceLevel {
    pub price: Uint128,
    pub quantity: Uint128,
    pub order_count: u32,
}

// Storage items
pub const CONFIG: Item<Config> = Item::new("config");
pub const TOKEN_PAIRS: Map<String, TokenPair> = Map::new("token_pairs");
pub const ORDER_BOOKS: Map<String, OrderBook> = Map::new("order_books");
pub const TRADES: Map<u64, Trade> = Map::new("trades");
pub const ORDERS: Map<u64, Order> = Map::new("orders");
pub const POOLS: Map<String, Pool> = Map::new("pools");
pub const NEXT_ORDER_ID: Item<u64> = Item::new("next_order_id");
pub const NEXT_TRADE_ID: Item<u64> = Item::new("next_trade_id");
pub const USER_ORDERS: Map<(Addr, u64), Order> = Map::new("user_orders");
pub const TOKEN_INFO: Map<String, TokenInfo> = Map::new("token_info");
pub const USER_TRADES: Map<(Addr, u64), Trade> = Map::new("user_trades");
pub const USER_TRADE_COUNT: Map<Addr, u64> = Map::new("user_trade_count");

// Add pruning timestamp
pub const LAST_PRUNED: Item<u64> = Item::new("last_pruned");
