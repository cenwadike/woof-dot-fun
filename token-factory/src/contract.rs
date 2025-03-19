#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError, StdResult,
};
use cw2::set_contract_version;
use execute::{
    execute_create_token, execute_transfer_ownership, execute_update_token_code_id,
    handle_token_creation_reply,
};
use query::{
    query_config, query_list_tokens, query_owner, query_token_address, query_token_count, query_token_info, query_tokens_by_creator
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{State, STATE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:token-factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let state = State {
        owner: info.sender.clone(),
        token_count: 0u32,
        token_code_id: msg.token_code_id,
        token_code_hash: msg.token_code_hash,
        token_creation_reply_id: 1u64,
    };

    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("token_code_id", msg.token_code_id.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateToken {
            name,
            symbol,
            decimals,
            uri,
            max_price_impact,
            curve_slope,
            initial_balances,
        } => Ok(execute_create_token(
            deps,
            env,
            info,
            name,
            symbol,
            decimals,
            uri,
            max_price_impact,
            curve_slope,
            initial_balances,
        )?),
        ExecuteMsg::TransferOwnership { new_owner } => {
            Ok(execute_transfer_ownership(deps, info, new_owner)?)
        }
        ExecuteMsg::UpdateTokenCodeId {
            new_token_code_id,
            new_token_code_hash,
        } => Ok(execute_update_token_code_id(
            deps,
            info,
            new_token_code_id,
            new_token_code_hash,
        )?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
    let state = STATE.load(deps.storage)?;
    let code = state.token_creation_reply_id;

    if msg.id == code {
        handle_token_creation_reply(deps, msg)
    } else {
        Err(StdError::generic_err(format!(
            "Unknown reply id: {}",
            msg.id
        )))
    }
}

pub mod execute {
    use crate::state::{Cw20Coin, TokenCreationResponse, TokenInfo, TOKEN_ADDRESS, TOKEN_INFO};
    use cosmwasm_std::{instantiate2_address, Addr, HexBinary, SubMsg, Uint128, WasmMsg};

    use super::*;

    pub fn execute_create_token(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        name: String,
        symbol: String,
        decimals: u8,
        uri: String,
        max_price_impact: Uint128, 
        curve_slope: Uint128,      
        initial_balances: Vec<Cw20Coin>,
    ) -> StdResult<Response> {
        let mut state = STATE.load(deps.storage)?;
    
        let is_unique = TOKEN_ADDRESS.has(deps.storage, (&name, &symbol));
        if !is_unique {
            return Err(StdError::generic_err("Token name and symbol pair must be unique"));
        }
    
        let sender = deps.api.addr_validate(info.sender.as_str())?;
        for coin in &initial_balances {
            deps.api.addr_validate(&coin.address)?;
        }
    
        let token_count = state.token_count + 1;
        let creator_canon = deps.api.addr_canonicalize(sender.as_str())?;
        let salt = Binary::new(format!("woof_token_{}", token_count).into_bytes());
    
        let address = instantiate2_address(&state.token_code_hash, &creator_canon, &salt).unwrap();
        let human_address = deps.api.addr_humanize(&address)?;
    
        let total_supply = initial_balances
            .iter()
            .try_fold(Uint128::zero(), |acc, coin| acc.checked_add(coin.amount))?;
    
        let token_info = TokenInfo {
            name: name.clone(),
            symbol: symbol.clone(),
            decimals,
            uri: uri.clone(),
            creator: sender.clone(),
            address: human_address.clone(),
            creation_time: env.block.time.seconds(),
            total_supply,
        };
    
        TOKEN_ADDRESS.save(deps.storage, (&name, &symbol), &human_address)?;
        TOKEN_INFO.save(deps.storage, &human_address.as_str(), &token_info)?;
        state.token_count = token_count;
        STATE.save(deps.storage, &state)?;
    
        let instantiate_msg = cw20_base::msg::InstantiateMsg {
            name: name.clone(),
            symbol: symbol.clone(),
            decimals,
            initial_balances: initial_balances
                .into_iter()
                .map(|coin| cw20::Cw20Coin {
                    address: coin.address,
                    amount: coin.amount,
                })
                .collect(),
            mint: None,
            marketing: None,
        };
    
        let instantiate = WasmMsg::Instantiate2 {
            admin: Some(sender.to_string()),
            code_id: state.token_code_id,
            label: format!("woof_token_{}", token_count),
            msg: to_json_binary(&instantiate_msg)?,
            funds: vec![],
            salt,
        };
    
        Ok(Response::new()
            .add_submessage(SubMsg::reply_on_success(instantiate, state.token_creation_reply_id))
            .add_attributes(vec![
                ("action", "create_token"),
                ("name", &name),
                ("symbol", &symbol),
                ("decimals", &decimals.to_string()),
                ("uri", &uri),
                ("address", human_address.as_str()),
                ("creator", sender.as_str()),
                ("max_price_impact", &max_price_impact.to_string()),
                ("curve_slope", &curve_slope.to_string()),
            ]))
    }

    pub fn execute_transfer_ownership(
        deps: DepsMut,
        info: MessageInfo,
        new_owner: Addr,
    ) -> StdResult<Response> {
        let mut state = STATE.load(deps.storage)?;
        let owner = state.owner;
        if info.sender != owner {
            return Err(StdError::generic_err("Unauthorized"));
        }

        state.owner = new_owner.clone();

        STATE.save(deps.storage, &state)?;

        Ok(Response::new().add_attributes(vec![
            ("action", "transfer_ownership"),
            ("new_owner", &new_owner.to_string()),
        ]))
    }

    pub fn execute_update_token_code_id(
        deps: DepsMut,
        info: MessageInfo,
        new_token_code_id: u64,
        new_token_code_hash: HexBinary,
    ) -> StdResult<Response> {
        let state = STATE.load(deps.storage)?;
        let owner = state.owner.clone();
        if info.sender != owner {
            return Err(StdError::generic_err("Unauthorized"));
        }

        STATE.save(
            deps.storage,
            &State {
                owner: state.owner,
                token_count: state.token_count,
                token_code_id: new_token_code_id,
                token_code_hash: new_token_code_hash,
                token_creation_reply_id: state.token_creation_reply_id,
            },
        )?;

        Ok(Response::new().add_attributes(vec![
            ("action", "update_token_code_id"),
            ("new_token_code_id", &new_token_code_id.to_string()),
        ]))
    }

    pub fn handle_token_creation_reply(deps: DepsMut, msg: Reply) -> StdResult<Response> {
        let res = cw_utils::parse_instantiate_response_data(&msg.payload).unwrap();
        let token_address = res.contract_address;    
        let token_info = TOKEN_INFO.load(deps.storage, &token_address)?;
    
        let response_data = TokenCreationResponse {
            token_address: token_address.clone(),
            name: token_info.name.clone(),
            symbol: token_info.symbol.clone(),
            decimals: token_info.decimals,
            max_price_impact: 100u128.into(),
            curve_slope: 500u128.into(),
        };
    
        Ok(Response::new()
            .add_attribute("token_address", token_address)
            .set_data(to_json_binary(&response_data)?))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetTokenAddress { name, symbol } => {
            to_json_binary(&query_token_address(deps, name, symbol)?)
        }
        QueryMsg::GetTokenInfo { address } => to_json_binary(&query_token_info(deps, address)?),
        QueryMsg::GetTokensByCreator { creator } => {
            to_json_binary(&query_tokens_by_creator(deps, creator)?)
        }
        QueryMsg::GetTokenCount {} => to_json_binary(&query_token_count(deps)?),
        QueryMsg::GetOwner {} => to_json_binary(&query_owner(deps)?),
        QueryMsg::GetListTokens { start_after, limit } => {
            to_json_binary(&query_list_tokens(deps, start_after, limit)?)
        }
        QueryMsg::GetConfig { } => {
            to_json_binary(&query_config(deps)?)
        }
    }
}

pub mod query {
    use crate::{
        msg::{
            GetConfigResponse, GetListTokensResponse, GetOwnerResponse, GetTokenAddressResponse, GetTokenCountResponse, GetTokenInfoResponse, GetTokensByCreatorResponse
        },
        state::{TokenInfo, DEFAULT_LIMIT, MAX_LIMIT, TOKEN_ADDRESS, TOKEN_INFO},
    };
    use cosmwasm_std::{Addr, Order};
    use cw_storage_plus::Bound;

    use super::*;

    pub fn query_token_address(
        deps: Deps,
        name: String,
        symbol: String,
    ) -> StdResult<GetTokenAddressResponse> {
        let address = TOKEN_ADDRESS.load(deps.storage, (&name, &symbol))?;

        Ok(GetTokenAddressResponse {
            address: Addr::unchecked(address),
        })
    }

    pub fn query_token_info(deps: Deps, address: String) -> StdResult<GetTokenInfoResponse> {
        let token_info = TOKEN_INFO.load(deps.storage, &address)?;

        Ok(GetTokenInfoResponse { token_info })
    }

    pub fn query_tokens_by_creator(
        deps: Deps,
        creator: Addr,
    ) -> StdResult<GetTokensByCreatorResponse> {
        let tokens: Vec<TokenInfo> = TOKEN_INFO
            .range(deps.storage, None, None, Order::Ascending)
            .filter_map(|item| {
                item.ok().and_then(|(_, token_info)| {
                    if token_info.creator == creator {
                        Some(token_info)
                    } else {
                        None
                    }
                })
            })
            .collect();

        Ok(GetTokensByCreatorResponse { tokens })
    }

    pub fn query_token_count(deps: Deps) -> StdResult<GetTokenCountResponse> {
        let state = STATE.load(deps.storage)?;
        let count = state.token_count;

        Ok(GetTokenCountResponse { count })
    }

    pub fn query_owner(deps: Deps) -> StdResult<GetOwnerResponse> {
        let state = STATE.load(deps.storage)?;
        let owner = state.owner;

        Ok(GetOwnerResponse { owner })
    }

    pub fn query_list_tokens(
        deps: Deps,
        start_from: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<GetListTokensResponse> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start = start_from.map(|s| s).unwrap();

        let tokens: Vec<TokenInfo> = TOKEN_INFO
            .range(
                deps.storage,
                Some(Bound::inclusive(&*start)),
                None,
                Order::Ascending,
            )
            .take(limit)
            .map(|item| item.map(|(_, token_info)| token_info))
            .collect::<StdResult<Vec<_>>>()?;

        Ok(GetListTokensResponse { tokens })
    }

    pub fn query_config(
        deps: Deps
    ) -> StdResult<GetConfigResponse> {
        let config = STATE.load(deps.storage)?;

        Ok(GetConfigResponse { config })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{ContractAddress, Cw20Coin, TokenInfo, TOKEN_INFO};
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cosmwasm_std::{attr, Addr, Event, HexBinary, SubMsgResponse, SubMsgResult, Uint128};
    use cw_multi_test::App;
    use cw_multi_test::ContractWrapper;
    use prost::Message;

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            token_code_id: 10,
            token_code_hash: HexBinary::default(),
        };
        let info = message_info(&Addr::unchecked("creator"), &[]);
        let env = mock_env();

        let res = instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("method", "instantiate"),
                attr("owner", info.sender.clone()),
                attr("token_code_id", "10"),
            ]
        );

        let state = STATE.load(&deps.storage).unwrap();
        assert_eq!(
            state,
            State {
                owner: info.sender,
                token_count: 0,
                token_code_id: 10,
                token_code_hash: HexBinary::default(),
                token_creation_reply_id: 1,
            }
        );
    }

    #[test]
    fn test_execute_create_token() {
        let mut deps = mock_dependencies();

        let mut app = App::default();

        let creator = app.api().addr_make("creator");

        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));

        // Instantiate the contract
        let instantiate_msg = InstantiateMsg {
            token_code_id: code_id,
            token_code_hash: HexBinary::from_hex(
                &"528E5F16D05CDE640CDEF6D779A458CBF566AA4820E40ACFCF5066978D388CAD",
            )
            .unwrap(),
        };
        let info = message_info(&Addr::unchecked(creator), &[]);
        let env = mock_env();
        instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap();

        // Prepare the CreateToken message
        let create_token_msg = ExecuteMsg::CreateToken {
            name: "Token".to_string(),
            symbol: "TKN".to_string(),
            decimals: 9,
            uri: "URL".to_string(),
            initial_balances: vec![Cw20Coin {
                address: env.contract.address.to_string(),
                amount: Uint128::new(1000 * 10_i32.pow(9) as u128),
            }],
            max_price_impact: 10u128.into(),
            curve_slope: 500u128.into(),
        };

        // Execute the CreateToken message
        let res = execute(deps.as_mut(), env.clone(), info.clone(), create_token_msg).unwrap();

        // Check response attributes and extract token address
        let token_address_attr = res
            .attributes
            .iter()
            .find(|&attr| attr.key == "address")
            .expect("Token address not found");
        let token_address_event = token_address_attr.value.clone();

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "create_token"),
                attr("name", "Token"),
                attr("symbol", "TKN"),
                attr("decimals", "9"),
                attr("uri", "URL"),
                attr("address", token_address_event.as_str()),
                attr("creator", info.sender.as_str()),
            ]
        );

        // Query the token address using the query function
        let query_res =
            query_token_address(deps.as_ref(), "Token".to_string(), "TKN".to_string()).unwrap();
        let token_address_query = query_res.address.to_string();

        // // Ensure the token address from the event matches the one queried from the blockchain
        assert_eq!(token_address_event, token_address_query);

        // Further checks on the token info
        let token_info = TOKEN_INFO
            .load(&deps.storage, token_address_query.as_str())
            .unwrap();
        assert_eq!(token_info.name, "Token".to_string());
        assert_eq!(token_info.symbol, "TKN".to_string());
        assert_eq!(token_info.decimals, 9);
        assert_eq!(token_info.uri, "URL".to_string());
        assert_eq!(
            token_info.total_supply,
            Uint128::new(1000 * 10_i32.pow(9) as u128)
        );
        assert_eq!(token_info.creator, info.sender);
    }

    #[test]
    fn test_execute_transfer_ownership() {
        let mut deps = mock_dependencies();

        let instantiate_msg = InstantiateMsg {
            token_code_id: 10,
            token_code_hash: HexBinary::default(),
        };
        let info = message_info(&Addr::unchecked("creator"), &[]);
        let env = mock_env();
        instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap();

        let new_owner = Addr::unchecked("new_owner");
        let transfer_ownership_msg = ExecuteMsg::TransferOwnership {
            new_owner: new_owner.clone(),
        };

        let res = execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            transfer_ownership_msg,
        )
        .unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "transfer_ownership"),
                attr("new_owner", new_owner.as_str()),
            ]
        );

        let state = STATE.load(&deps.storage).unwrap();
        assert_eq!(state.owner, new_owner);
    }

    #[test]
    fn test_execute_update_token_code_id() {
        let mut deps = mock_dependencies();

        let instantiate_msg = InstantiateMsg {
            token_code_id: 10,
            token_code_hash: HexBinary::default(),
        };
        let info = message_info(&Addr::unchecked("creator"), &[]);
        let env = mock_env();
        instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap();

        let update_token_code_id_msg = ExecuteMsg::UpdateTokenCodeId {
            new_token_code_id: 20,
            new_token_code_hash: HexBinary::default(),
        };

        let res = execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            update_token_code_id_msg,
        )
        .unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "update_token_code_id"),
                attr("new_token_code_id", "20"),
            ]
        );

        let state = STATE.load(&deps.storage).unwrap();
        assert_eq!(state.token_code_id, 20);
    }

    #[test]
    #[allow(deprecated)]
    fn test_reply() {
        let mut deps = mock_dependencies();

        let instantiate_msg = InstantiateMsg {
            token_code_id: 10,
            token_code_hash: HexBinary::default(),
        };
        let info = message_info(&Addr::unchecked("creator"), &[]);
        let env = mock_env();
        instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap();

        // Create a Protobuf message for the payload
        let contract_address_msg = ContractAddress {
            contract_address: "woof1abcd".to_string(),
        };

        let mut buf = Vec::new();
        contract_address_msg.encode(&mut buf).unwrap();

        // Create a SubMsgResponse with msg_responses for newer versions and payload for the older version
        let msg_response = SubMsgResponse {
            events: vec![Event::new("instantiate").add_attribute("contract_address", "woof1abcd")],
            data: Some(Binary::from(buf.clone())), // This is deprecated but required
            msg_responses: vec![],
        };

        // Create Reply message including the payload
        let msg = Reply {
            id: 1,
            result: SubMsgResult::Ok(msg_response),
            payload: Binary::from(buf),
            gas_used: 1000,
        };

        let res = reply(deps.as_mut(), env.clone(), msg).unwrap();
        assert_eq!(res.attributes, vec![attr("token_address", "woof1abcd"),]);
    }

    #[test]
    fn test_query_token_info() {
        let mut deps = mock_dependencies();
        let token_address = "woof1abcd".to_string();

        let token_info = TokenInfo {
            name: "Token".to_string(),
            symbol: "TKN".to_string(),
            decimals: 9,
            uri: "URL".to_string(),
            creator: Addr::unchecked("creator"),
            address: Addr::unchecked(token_address.clone()),
            creation_time: 1234567890,
            total_supply: Uint128::new(1000),
        };

        TOKEN_INFO
            .save(&mut deps.storage, &token_address, &token_info)
            .unwrap();

        let response = query::query_token_info(deps.as_ref(), token_address.clone()).unwrap();
        assert_eq!(response.token_info, token_info);
    }

    #[test]
    fn test_query_tokens_by_creator() {
        let mut deps = mock_dependencies();

        let token_info1 = TokenInfo {
            name: "Token1".to_string(),
            symbol: "TK1".to_string(),
            decimals: 8,
            uri: "URL".to_string(),
            creator: Addr::unchecked("creator"),
            address: Addr::unchecked("woof1abcd"),
            creation_time: 1234567890,
            total_supply: Uint128::new(1000),
        };

        let token_info2 = TokenInfo {
            name: "Token2".to_string(),
            symbol: "TK2".to_string(),
            decimals: 8,
            uri: "URL".to_string(),
            creator: Addr::unchecked("creator"),
            address: Addr::unchecked("woof1efgh"),
            creation_time: 1234567890,
            total_supply: Uint128::new(2000),
        };

        TOKEN_INFO
            .save(
                &mut deps.storage,
                token_info1.address.as_str(),
                &token_info1,
            )
            .unwrap();
        TOKEN_INFO
            .save(
                &mut deps.storage,
                token_info2.address.as_str(),
                &token_info2,
            )
            .unwrap();

        let response =
            query::query_tokens_by_creator(deps.as_ref(), Addr::unchecked("creator")).unwrap();
        assert_eq!(response.tokens.len(), 2);
        assert!(response.tokens.contains(&token_info1));
        assert!(response.tokens.contains(&token_info2));
    }

    #[test]
    fn test_query_token_count() {
        let mut deps = mock_dependencies();
        let state = State {
            owner: Addr::unchecked("owner"),
            token_count: 2,
            token_code_id: 1,
            token_code_hash: HexBinary::default(),
            token_creation_reply_id: 1,
        };
        STATE.save(&mut deps.storage, &state).unwrap();

        let response = query::query_token_count(deps.as_ref()).unwrap();
        assert_eq!(response.count, 2);
    }

    #[test]
    fn test_query_owner() {
        let mut deps = mock_dependencies();
        let state = State {
            owner: Addr::unchecked("owner"),
            token_count: 2,
            token_code_id: 1,
            token_code_hash: HexBinary::default(),
            token_creation_reply_id: 1,
        };
        STATE.save(&mut deps.storage, &state).unwrap();

        let response = query::query_owner(deps.as_ref()).unwrap();
        assert_eq!(response.owner, Addr::unchecked("owner"));
    }

    #[test]
    fn test_query_list_tokens() {
        let mut deps = mock_dependencies();

        let token_info1 = TokenInfo {
            name: "Token1".to_string(),
            symbol: "TK1".to_string(),
            decimals: 9,
            uri: "URL".to_string(),
            creator: Addr::unchecked("creator"),
            address: Addr::unchecked("woof1abcd".to_string()),
            creation_time: 1234567890,
            total_supply: Uint128::new(1000),
        };

        let token_info2 = TokenInfo {
            name: "Token2".to_string(),
            symbol: "TK2".to_string(),
            decimals: 9,
            uri: "URL".to_string(),
            creator: Addr::unchecked("creator"),
            address: Addr::unchecked("woof1efgh".to_string()),
            creation_time: 1234567890,
            total_supply: Uint128::new(2000),
        };

        TOKEN_INFO
            .save(
                &mut deps.storage,
                token_info1.address.as_str(),
                &token_info1,
            )
            .unwrap();
        TOKEN_INFO
            .save(
                &mut deps.storage,
                token_info2.address.as_str(),
                &token_info2,
            )
            .unwrap();

        let response =
            query::query_list_tokens(deps.as_ref(), Some("".to_string()), Some(2)).unwrap();
        assert_eq!(response.tokens.len(), 2);
        assert!(response.tokens.contains(&token_info1));
        assert!(response.tokens.contains(&token_info2));
    }
}
