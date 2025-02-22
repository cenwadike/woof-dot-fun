use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};

use crate::state::{
    Config, Order, OrderStatus, OrderType, Pool, PriceLevel, TokenInfo, TokenPair, Trade,
};

#[cw_serde]
pub struct InstantiateMsg {
    pub token_factory: Addr,
    pub fee_collector: Addr,
    pub quote_token_total_supply: Uint128,
    pub bonding_curve_supply: Uint128,
    pub lp_supply: Uint128,
    pub maker_fee: Decimal,
    pub taker_fee: Decimal,
    pub secondary_amm_address: Addr,
    pub base_token_denom: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    CreateToken {
        name: String,
        symbol: String,
        decimals: u8,
        max_price_impact: Uint128,
        curve_slope: Uint128,
    },
    Graduate {
        token_address: String,
    },
    PlaceLimitOrder {
        token_address: String,
        amount: Uint128,
        price: Uint128,
        is_buy: bool,
    },
    CancelOrder {
        order_id: u64,
        pair_id: String,
    },
    Swap {
        pair_id: String,
        token_address: String,
        amount: Uint128,
        min_return: Uint128,
        order_type: OrderType,
    },
    UpdateConfig {
        token_factory: Option<Addr>,
        fee_collector: Option<Addr>,
        maker_fee: Option<Decimal>,
        taker_fee: Option<Decimal>,
        quote_token_total_supply: Option<Uint128>,
        bonding_curve_supply: Option<Uint128>,
        lp_supply: Option<Uint128>,
        enabled: Option<bool>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // User queries
    #[returns(GetUserTradesResponse)]
    GetUserTrades {
        address: Addr,
        pair_id: Option<String>,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    #[returns(GetUserOrdersResponse)]
    GetUserOrders {
        address: Addr,
        pair_id: Option<String>,
        status: Option<OrderStatus>,
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    #[returns(GetCountResponse)]
    GetUserTradeCount { address: Addr },

    // Order book queries
    #[returns(GetOrderResponse)]
    GetOrder { order_id: u64 },

    // Order book queries
    #[returns(GetOrderBookResponse)]
    GetOrderBook {
        pair_id: String,
        depth: Option<u32>, // How many price levels to return
    },

    // Bonding curve pool queries and liquidity queries
    #[returns(GetPoolResponse)]
    GetPool { token_address: String },

    // Token queries
    #[returns(GetTokenInfoResponse)]
    GetTokenInfo { token_address: String },

    // Price queries
    #[returns(GetCurrentPriceResponse)]
    GetCurrentPrice { token_address: String },

    // Market data queries
    #[returns(GetRecentTradesResponse)]
    GetRecentTrades {
        start_from: Option<u64>,
        limit: Option<u32>,
    },

    // Token and pair queries
    #[returns(GetTokenPairResponse)]
    GetTokenPair { pair_id: String },
    #[returns(ListTokenPairsResponse)]
    ListTokenPairs {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    // System queries
    #[returns(GetSystemStatsResponse)]
    GetSystemStats {},

    #[returns(GetConfigResponse)]
    GetConfig {},
}

// We define a custom struct for each query response
#[cw_serde]
pub struct GetCountResponse {
    pub count: u64,
}

#[cw_serde]
pub struct GetUserTradesResponse {
    pub trades: Vec<Trade>,
}

#[cw_serde]
pub struct GetUserOrdersResponse {
    pub orders: Vec<Order>,
}

#[cw_serde]
pub struct GetConfigResponse {
    pub config: Config,
}

#[cw_serde]
pub struct GetOrderResponse {
    pub order: Order,
}

#[cw_serde]
pub struct GetOrderBookResponse {
    pub pair_id: String,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub last_price: Uint128,
    pub base_volume_24h: Uint128,
    pub quote_volume_24h: Uint128,
}

#[cw_serde]
pub struct GetPoolResponse {
    pub pool: Pool,
}

#[cw_serde]
pub struct GetCurrentPriceResponse {
    pub price: u128,
}

#[cw_serde]
pub struct GetTokenInfoResponse {
    pub token_info: TokenInfo,
}

#[cw_serde]
pub struct GetRecentTradesResponse {
    pub trades: Vec<Trade>,
}

#[cw_serde]
pub struct GetTokenPairResponse {
    pub token_pair: TokenPair,
}

#[cw_serde]
pub struct ListTokenPairsResponse {
    pub token_pairs: Vec<TokenPair>,
}

#[cw_serde]
pub struct GetSystemStatsResponse {
    pub total_pairs: u64,
    pub total_orders: u64,
    pub total_trades: u64,
    pub total_volume: Uint128,
    pub total_users: u64,
    pub total_fees_collected: Uint128,
}
