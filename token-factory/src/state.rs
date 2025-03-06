use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, HexBinary, Uint128};
use cw_storage_plus::{Item, Map};

pub const DEFAULT_LIMIT: u32 = 10;
pub const MAX_LIMIT: u32 = 30;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct State {
    pub owner: Addr,
    pub token_count: u32,
    pub token_code_id: u64,
    pub token_code_hash: HexBinary,
    pub token_creation_reply_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Cw20Coin {
    pub address: String,
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub uri: String,
    pub creator: Addr,
    pub address: Addr,
    pub creation_time: u64,
    pub total_supply: Uint128,
}

pub const STATE: Item<State> = Item::new("state");
pub const TOKEN_INFO: Map<&str, TokenInfo> = Map::new("token_info");
pub const TOKEN_ADDRESS: Map<(&str, &str), Addr> = Map::new("token_address");

use prost::Message;

#[derive(Clone, PartialEq, Message)]
pub struct ContractAddress {
    #[prost(string, tag = "1")]
    pub contract_address: String,
}
