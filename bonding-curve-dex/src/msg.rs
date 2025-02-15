use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};

use crate::state::{OrderType, TimeFrame};

#[cw_serde]
pub struct InstantiateMsg {
    pub token_factory: Addr,
    pub fee_collector: Addr,
    pub trading_fee_rate: Decimal,
    pub quote_token_total_supply: Uint128,
    pub bonding_curve_supply: Uint128,
    pub lp_supply: Uint128,
    pub maker_fee: Uint128,
    pub taker_fee: Uint128,
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
        trading_fee_rate: Option<Decimal>,
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
    #[returns(GetCountResponse)]
    GetTradeHistory {
        token_address: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    #[returns(GetCountResponse)]
    GetUserOrders {
        address: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    // Protocol queries
    #[returns(GetCountResponse)]
    GetConfig {},

    // Order book queries
    #[returns(GetCountResponse)]
    GetOrderBook { token_address: String },
    #[returns(GetCountResponse)]
    GetOrder { order_id: u64 },

    // Binding curve pool queries
    #[returns(GetCountResponse)]
    GetPoolInfo { token_address: String },

    // Market queries
    #[returns(GetCountResponse)]
    GetMarketData {
        token_address: String,
        time_frame: TimeFrame,
    },

    // Token queries
    #[returns(GetCountResponse)]
    GetTokenPrice { token_address: String },
    #[returns(GetCountResponse)]
    GetTokenInfo { token_address: String },
}

// We define a custom struct for each query response
#[cw_serde]
pub struct GetCountResponse {
    pub count: i32,
}
