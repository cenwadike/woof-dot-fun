use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, HexBinary};

use crate::state::{Cw20Coin, TokenInfo};

#[cw_serde]
pub struct InstantiateMsg {
    pub token_code_id: u64,
    pub token_code_hash: HexBinary,
}

#[cw_serde]
pub enum ExecuteMsg {
    CreateToken {
        name: String,
        symbol: String,
        decimals: u8,
        uri: String,
        initial_balances: Vec<Cw20Coin>,
    },
    TransferOwnership {
        new_owner: Addr,
    },
    UpdateTokenCodeId {
        new_token_code_id: u64,
        new_token_code_hash: HexBinary,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetTokenAddressResponse)]
    GetTokenAddress { name: String, symbol: String },
    #[returns(GetTokenInfoResponse)]
    GetTokenInfo { address: String },
    #[returns(GetTokensByCreatorResponse)]
    GetTokensByCreator { creator: Addr },
    #[returns(GetTokenCountResponse)]
    GetTokenCount {},
    #[returns(GetOwnerResponse)]
    GetOwner {},
    #[returns(GetListTokensResponse)]
    GetListTokens {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

// We define a custom struct for each query response
#[cw_serde]
pub struct GetTokenAddressResponse {
    pub address: Addr,
}

#[cw_serde]
pub struct GetTokenInfoResponse {
    pub token_info: TokenInfo,
}

#[cw_serde]
pub struct GetTokensByCreatorResponse {
    pub tokens: Vec<TokenInfo>,
}

#[cw_serde]
pub struct GetTokenCountResponse {
    pub count: u32,
}

#[cw_serde]
pub struct GetOwnerResponse {
    pub owner: Addr,
}

#[cw_serde]
pub struct GetListTokensResponse {
    pub tokens: Vec<TokenInfo>,
}
