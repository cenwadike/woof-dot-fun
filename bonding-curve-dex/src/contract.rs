#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Decimal, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128,
};
use cw2::set_contract_version;
use execute::{
    execute_cancel_order, execute_create_token, execute_graduate, execute_place_limit_order,
    execute_swap, execute_update_config,
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{Config, OrderBook, CONFIG, NEXT_ORDER_ID, NEXT_TRADE_ID};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:bonding-curve-dex";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate non-empty addresses
    if msg.token_factory.to_string().is_empty()
        || msg.fee_collector.to_string().is_empty()
        || msg.secondary_amm_address.to_string().is_empty()
    {
        return Err(StdError::generic_err("Invalid address provided."));
    }

    // Validate non-negative values
    if msg.trading_fee_rate < Decimal::zero()
        || msg.quote_token_total_supply.is_zero()
        || msg.bonding_curve_supply.is_zero()
        || msg.lp_supply.is_zero()
        || msg.maker_fee.is_zero()
        || msg.taker_fee.is_zero()
    {
        return Err(StdError::generic_err(
            "Invalid input: zero or negative values are not allowed.",
        ));
    }

    // Validate that the trading fee rate is within acceptable range
    if msg.trading_fee_rate > Decimal::one() {
        return Err(StdError::generic_err(
            "Trading fee rate must be between 0 and 1.",
        ));
    }

    // Validate that the base token denomination is not empty
    if msg.base_token_denom.is_empty() {
        return Err(StdError::generic_err(
            "Base token denomination must not be empty.",
        ));
    }

    let config = Config {
        owner: info.sender.clone(),
        token_factory: msg.token_factory.clone(),
        fee_collector: msg.fee_collector.clone(),
        trading_fee: msg.trading_fee_rate.clone(),
        enabled: true,
        quote_token_total_supply: msg.quote_token_total_supply.clone().into(),
        bonding_curve_supply: msg.bonding_curve_supply.clone().into(),
        lp_supply: msg.lp_supply.clone().into(),
        maker_fee: Decimal::from_ratio(msg.maker_fee.clone(), Uint128::one()),
        taker_fee: Decimal::from_ratio(msg.taker_fee.clone(), Uint128::one()),
        secondary_amm_address: msg.secondary_amm_address.clone(),
        base_token_denom: msg.base_token_denom.clone(),
    };

    CONFIG.save(deps.storage, &config)?;
    NEXT_ORDER_ID.save(deps.storage, &0u64)?;
    NEXT_TRADE_ID.save(deps.storage, &0u64)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("token_factory", msg.token_factory.to_string())
        .add_attribute("trading_fee_rate", msg.trading_fee_rate.to_string())
        .add_attribute("fee_collector", msg.fee_collector.to_string())
        .add_attribute(
            "quote_token_total_supply",
            msg.quote_token_total_supply.to_string(),
        )
        .add_attribute("bonding_curve_supply", msg.bonding_curve_supply.to_string())
        .add_attribute("lp_supply", msg.lp_supply.to_string())
        .add_attribute("maker_fee", msg.maker_fee.to_string())
        .add_attribute("taker_fee", msg.taker_fee.to_string())
        .add_attribute(
            "secondary_amm_address",
            msg.secondary_amm_address.to_string(),
        )
        .add_attribute("base_token_denom", msg.base_token_denom))
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
            max_price_impact,
            curve_slope,
        } => Ok(execute_create_token(
            deps,
            env,
            info,
            name,
            symbol,
            decimals,
            max_price_impact,
            curve_slope,
        )?),
        ExecuteMsg::PlaceLimitOrder {
            token_address,
            amount,
            price,
            is_buy,
        } => Ok(execute_place_limit_order(
            deps,
            env,
            info,
            token_address,
            amount,
            price,
            is_buy,
        )?),
        ExecuteMsg::CancelOrder { order_id, pair_id } => {
            Ok(execute_cancel_order(deps, env, info, order_id, pair_id)?)
        }
        ExecuteMsg::Swap {
            pair_id,
            token_address,
            amount,
            min_return,
            order_type,
        } => Ok(execute_swap(
            deps,
            env,
            info,
            pair_id,
            token_address,
            amount,
            min_return,
            order_type,
        )?),
        ExecuteMsg::UpdateConfig {
            token_factory,
            fee_collector,
            trading_fee_rate,
            quote_token_total_supply,
            bonding_curve_supply,
            lp_supply,
            enabled,
        } => Ok(execute_update_config(
            deps,
            env,
            info,
            token_factory,
            fee_collector,
            trading_fee_rate,
            quote_token_total_supply,
            bonding_curve_supply,
            lp_supply,
            enabled,
        )?),
        ExecuteMsg::Graduate { token_address } => {
            Ok(execute_graduate(deps, env, info, token_address)?)
        }
    }
}

pub mod execute {

    use std::str::FromStr;

    use cosmwasm_std::{
        attr, Addr, BankMsg, Coin, CosmosMsg, Decimal, Deps, StdError, Storage, Uint128, WasmMsg,
    };
    use cw20::Cw20ExecuteMsg;
    use token_factory::{contract::execute::generate_token_address, state::Cw20Coin};

    use crate::state::{
        Order, OrderStatus, OrderType, Pool, TokenInfo, TokenPair, Trade, BASE_PRICE,
        MAX_TRADES_PER_USER, ORDER_BOOKS, POOLS, TOKEN_INFO, TOKEN_PAIRS, TRADES, USER_ORDERS,
        USER_TRADES, USER_TRADE_COUNT,
    };

    use super::*;

    pub fn execute_create_token(
        deps: DepsMut,
        env: Env,
        _info: MessageInfo,
        name: String,
        symbol: String,
        decimals: u8,
        max_price_impact: Uint128,
        curve_slope: Uint128,
    ) -> StdResult<Response> {
        let config = CONFIG.load(deps.storage)?;

        // Validate input values
        if name.is_empty() || symbol.is_empty() {
            return Err(StdError::generic_err(
                "Token name and symbol must not be empty.",
            ));
        }

        if decimals == 0 {
            return Err(StdError::generic_err("Decimals must be greater than 0."));
        }

        if max_price_impact.is_zero() {
            return Err(StdError::generic_err(
                "Max price impact must be greater than 0.",
            ));
        }

        if curve_slope.is_zero() {
            return Err(StdError::generic_err("Curve slope must be greater than 0."));
        }

        let total_supply = config
            .quote_token_total_supply
            .checked_mul(Uint128::from(10u128.pow(decimals as u32)).into())
            .unwrap_or(0u128);
        let token_address = generate_token_address(&name, &symbol);

        let token_info = TokenInfo {
            name: name.clone(),
            symbol: symbol.clone(),
            decimals,
            total_supply: total_supply.into(),
            initial_price: BASE_PRICE.into(),
            max_price_impact,
            graduated: false,
        };

        let token_pair = TokenPair {
            base_token: config.base_token_denom.clone(),
            quote_token: token_address.clone().to_string(),
            base_decimals: 6,
            quote_decimals: token_info.decimals,
            enabled: true,
        };

        // remove the "u" before the token
        let pair_id = format!("{}/{}", token_info.symbol, &config.base_token_denom[1..]);

        // Call token factory contract
        let msg = WasmMsg::Execute {
            contract_addr: config.token_factory.to_string(),
            msg: to_json_binary(&token_factory::msg::ExecuteMsg::CreateToken {
                name: name.clone(),
                symbol: symbol.clone(),
                decimals,
                initial_balances: vec![Cw20Coin {
                    address: env.contract.address.to_string(),
                    amount: total_supply.into(),
                }],
            })?,
            funds: vec![],
        };

        // Initialize pool
        let pool = Pool {
            token_address: token_address.clone(),
            total_reserve_token: Uint128::zero(),
            token_sold: Uint128::zero(),
            total_volume: Uint128::zero(),
            total_fees_collected: Uint128::zero(),
            curve_slope,
            pair_id: pair_id.clone(),
            total_trades: Uint128::zero(),
            last_price: BASE_PRICE.into(),
            enabled: true,
        };

        TOKEN_INFO.save(deps.storage, token_address.to_string(), &token_info)?;
        TOKEN_PAIRS.save(deps.storage, pair_id, &token_pair)?;
        POOLS.save(deps.storage, token_address.to_string(), &pool)?;

        Ok(Response::new()
            .add_message(msg)
            .add_attribute("action", "create_token")
            .add_attribute("name", name)
            .add_attribute("symbol", symbol)
            .add_attribute("decimals", decimals.to_string())
            .add_attribute("max_price_impact", max_price_impact.to_string())
            .add_attribute("curve_slope", curve_slope.to_string()))
    }

    pub fn execute_place_limit_order(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        pair_id: String,
        amount: Uint128,
        price: Uint128,
        is_buy: bool,
    ) -> StdResult<Response> {
        let config = CONFIG.load(deps.storage)?;
        if !config.enabled {
            return Err(StdError::generic_err("Trading is currently disabled"));
        }

        // Load token pair and validate it exists and is enabled
        let token_pair = TOKEN_PAIRS.load(deps.storage, pair_id.clone())?;
        if !token_pair.enabled {
            return Err(StdError::generic_err("Trading pair is disabled"));
        }

        // Check if tokens were sent and handle token transfers
        validate_and_handle_tokens(&deps, &env, &info, &token_pair, amount, price, is_buy)?;

        let mut order_book = ORDER_BOOKS.load(deps.storage, pair_id.clone())?;
        let next_id = NEXT_ORDER_ID.load(deps.storage)?;

        // Create new order
        let order = Order {
            id: next_id,
            owner: info.sender.clone(),
            pair_id: pair_id.clone(),
            token_amount: amount,
            price,
            timestamp: env.block.time.seconds() as u64,
            status: OrderStatus::Active,
            filled_amount: Uint128::zero(),
            remaining_amount: amount,
            order_type: if is_buy {
                OrderType::Buy
            } else {
                OrderType::Sell
            },
            created_at: env.block.height,
        };

        // Add to order book
        if is_buy {
            order_book
                .buy_orders
                .entry(price.u128())
                .or_insert_with(Vec::new)
                .push(order.clone());
        } else {
            order_book
                .sell_orders
                .entry(price.u128())
                .or_insert_with(Vec::new)
                .push(order.clone());
        }

        // Save updated state
        ORDER_BOOKS.save(deps.storage, pair_id.clone(), &order_book)?;
        NEXT_ORDER_ID.save(deps.storage, &(next_id + 1))?;
        USER_ORDERS.save(deps.storage, (info.sender.clone(), next_id), &order)?;

        // Try to match orders
        match_orders(deps, &env, pair_id.clone(), is_buy)?;

        Ok(Response::new()
            .add_attribute("action", "place_limit_order")
            .add_attribute("order_id", next_id.to_string())
            .add_attribute("pair_id", pair_id)
            .add_attribute("is_buy", is_buy.to_string())
            .add_attribute("amount", amount.to_string())
            .add_attribute("price", price.to_string()))
    }

    pub fn execute_cancel_order(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        order_id: u64,
        pair_id: String,
    ) -> StdResult<Response> {
        let mut order_book = ORDER_BOOKS.load(deps.storage, pair_id.clone())?;

        // Check if the order exists
        let order = USER_ORDERS.may_load(deps.storage, (info.sender.clone(), order_id))?;
        let order = match order {
            Some(order) => order,
            None => return Err(StdError::not_found("Order")),
        };

        // Verify order ownership
        if order.owner != info.sender {
            return Err(StdError::generic_err("Unauthorized: not order owner"));
        }

        // Find and remove order from order book
        let removed_order = if order_book.buy_orders.contains_key(&order.price.u128()) {
            order_book
                .buy_orders
                .get_mut(&order.price.u128())
                .and_then(|orders| orders.iter().position(|o| o.id == order_id))
                .map(|index| {
                    order_book
                        .buy_orders
                        .get_mut(&order.price.u128())
                        .unwrap()
                        .remove(index)
                })
        } else if order_book.sell_orders.contains_key(&order.price.u128()) {
            order_book
                .sell_orders
                .get_mut(&order.price.u128())
                .and_then(|orders| orders.iter().position(|o| o.id == order_id))
                .map(|index| {
                    order_book
                        .sell_orders
                        .get_mut(&order.price.u128())
                        .unwrap()
                        .remove(index)
                })
        } else {
            return Err(StdError::generic_err("Order not found in order book"));
        };

        if removed_order.is_none() {
            return Err(StdError::generic_err("Order not found in order book"));
        }

        // Update order status
        let mut updated_order = order;
        updated_order.status = OrderStatus::Cancelled;
        USER_ORDERS.save(
            deps.storage,
            (info.sender.clone(), order_id),
            &updated_order,
        )?;
        ORDER_BOOKS.save(deps.storage, pair_id, &order_book)?;

        Ok(Response::new()
            .add_attribute("action", "cancel_order")
            .add_attribute("order_id", order_id.to_string()))
    }

    // Function to execute limit orders before using bonding curve
    pub fn execute_swap(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        pair_id: String,
        token_address: String,
        amount: Uint128,
        min_return: Uint128,
        order_type: OrderType,
    ) -> StdResult<Response> {
        let config = CONFIG.load(deps.storage)?;
        if !config.enabled {
            return Err(StdError::generic_err("Trading is currently disabled"));
        }

        let mut response = Response::new();

        // Try to match with limit orders first
        let (matched_amount, remaining_amount, match_response) = match_limit_orders(
            deps.storage,
            &info,
            &env,
            pair_id.clone(),
            amount,
            &order_type,
            min_return,
        )?;
        response = response.add_attributes(match_response.attributes);

        // If there's remaining amount, use bonding curve
        if !remaining_amount.is_zero() {
            let curve_response = execute_bonding_curve_swap(
                deps,
                env,
                info,
                pair_id,
                token_address,
                remaining_amount,
                min_return - remaining_amount,
                order_type,
            )?;
            response = response.add_attributes(curve_response.attributes);
        }

        Ok(response
            .add_attribute("matched_amount", matched_amount)
            .add_attribute("remaining_amount", remaining_amount))
    }

    pub fn execute_graduate(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        token_address: String,
    ) -> StdResult<Response> {
        // Load config and token info
        let config = CONFIG.load(deps.storage)?;
        let mut token_info = TOKEN_INFO.load(deps.storage, token_address.clone())?;
        let pool = POOLS.load(deps.storage, token_address.clone())?;

        // Verify caller is contract admin
        if info.sender != config.owner {
            return Err(StdError::generic_err("Unauthorized"));
        }

        // Check if token is eligible for graduation
        if token_info.graduated {
            return Err(StdError::generic_err("Token already graduated"));
        }

        if pool.token_sold != Uint128::from(config.bonding_curve_supply) {
            return Err(StdError::generic_err("Some token have not been sold"));
        }

        // Prepare messages for secondary AMM interaction
        let mut messages: Vec<CosmosMsg> = vec![];

        // Approve secondary AMM to spend tokens
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: token_address.clone(),
            msg: to_json_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                spender: config.secondary_amm_address.to_string(),
                amount: config.lp_supply.into(),
                expires: None,
            })?,
            funds: vec![],
        }));

        // // Add liquidity to secondary AMM
        // messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        //     contract_addr: config.secondary_amm_address.to_string(),
        //     msg: to_json_binary(&SecondaryAmmMsg::AddLiquidity {
        //         token_address: token_address.clone(),
        //         token_amount: config.lp_supply,
        //         min_liquidity: Uint128::zero(),
        //         max_tokens: pool.total_reserve_token,
        //         expiration: None,
        //     })?,
        //     funds: vec![Coin {
        //         denom: "uhuahua",
        //         amount: pool.total_reserve_token,
        //     }],
        // }));

        // Disable trading in bonding curve
        token_info.graduated = true;
        TOKEN_INFO.save(deps.storage, token_address.clone(), &token_info)?;

        // Remove pool from bonding curve
        POOLS.remove(deps.storage, token_address.clone());

        // 5. Emit graduation event
        Ok(Response::new()
            .add_messages(messages)
            .add_attribute("action", "graduate")
            .add_attribute("token", token_address)
            .add_attribute("secondary_amm", config.secondary_amm_address))
    }

    // Helper function to check if token has graduated
    pub fn is_token_graduated(storage: &dyn Storage, token_address: &str) -> StdResult<bool> {
        let token_info = TOKEN_INFO.load(storage, token_address.to_string())?;
        Ok(token_info.graduated)
    }

    pub fn execute_update_config(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        token_factory: Option<Addr>,
        fee_collector: Option<Addr>,
        trading_fee_rate: Option<Decimal>,
        quote_token_total_supply: Option<Uint128>,
        bonding_curve_supply: Option<Uint128>,
        lp_supply: Option<Uint128>,
        enabled: Option<bool>,
    ) -> StdResult<Response> {
        let mut config = CONFIG.load(deps.storage)?;

        // Verify authority
        if info.sender != config.owner {
            return Err(StdError::generic_err("Unauthorized"));
        }

        // Update fee configuration
        if token_factory.is_some() {
            config.token_factory = token_factory.unwrap();
        }

        if fee_collector.is_some() {
            config.fee_collector = fee_collector.unwrap();
        }

        if trading_fee_rate.is_some() {
            config.trading_fee = trading_fee_rate.unwrap();
        }

        if quote_token_total_supply.is_some() {
            config.quote_token_total_supply = quote_token_total_supply.unwrap().into();
        }

        if bonding_curve_supply.is_some() {
            config.bonding_curve_supply = bonding_curve_supply.unwrap().into();
        }

        if lp_supply.is_some() {
            config.lp_supply = lp_supply.unwrap().into();
        }

        if enabled.is_some() {
            config.enabled = enabled.unwrap().into();
        }

        CONFIG.save(deps.storage, &config)?;

        Ok(Response::new().add_attribute("action", "update_config"))
    }

    fn match_orders(
        deps: DepsMut,
        env: &Env,
        pair_id: String,
        is_buy: bool,
    ) -> StdResult<Response> {
        let mut order_book = ORDER_BOOKS.load(deps.storage, pair_id.clone())?;
        let config = CONFIG.load(deps.storage)?;
        let token_pair = TOKEN_PAIRS.load(deps.storage, pair_id.clone())?;
        let mut next_trade_id = NEXT_TRADE_ID.load(deps.storage)?;

        let mut response = Response::new();

        if is_buy {
            for (buy_price, buy_orders) in order_book.buy_orders.iter_mut().rev() {
                for buy_order in buy_orders.iter_mut() {
                    if buy_order.remaining_amount.is_zero() {
                        continue;
                    }

                    for (sell_price, sell_orders) in order_book.sell_orders.iter_mut() {
                        if sell_price > buy_price {
                            break;
                        }

                        for sell_order in sell_orders.iter_mut() {
                            if sell_order.remaining_amount.is_zero() {
                                continue;
                            }

                            // Create trade and merge responses
                            let trade_response = create_order(
                                deps.storage,
                                buy_order,
                                sell_order,
                                next_trade_id,
                                &token_pair,
                                &config,
                                env,
                            )?;

                            response = response.add_attributes(trade_response.attributes);
                            next_trade_id += 1;
                        }
                    }
                }
            }
        } else {
            for (sell_price, sell_orders) in order_book.sell_orders.iter_mut() {
                for sell_order in sell_orders.iter_mut() {
                    if sell_order.remaining_amount.is_zero() {
                        continue;
                    }

                    for (buy_price, buy_orders) in order_book.buy_orders.iter_mut().rev() {
                        if buy_price < sell_price {
                            break;
                        }

                        for buy_order in buy_orders.iter_mut() {
                            if buy_order.remaining_amount.is_zero() {
                                continue;
                            }

                            // Create trade and merge responses
                            let trade_response = create_order(
                                deps.storage,
                                buy_order,
                                sell_order,
                                next_trade_id,
                                &token_pair,
                                &config,
                                env,
                            )?;

                            response = response.add_attributes(trade_response.attributes);
                            next_trade_id += 1;
                        }
                    }
                }
            }
        }

        // Clean up filled orders
        clean_up_order_book(&mut order_book);

        // Save updated state
        ORDER_BOOKS.save(deps.storage, pair_id, &order_book)?;
        NEXT_TRADE_ID.save(deps.storage, &next_trade_id)?;

        Ok(response)
    }

    fn validate_and_handle_tokens(
        deps: &DepsMut,
        env: &Env,
        info: &MessageInfo,
        token_pair: &TokenPair,
        amount: Uint128,
        price: Uint128,
        is_buy: bool,
    ) -> StdResult<()> {
        if is_buy {
            let total_price = price * amount;
            let denom = token_pair.base_token.clone();
            validate_native_token_payment(info, &denom, total_price)?;
        } else {
            // Handle native token cases for sell orders
            validate_cw20_token_payment(
                &deps.as_ref(),
                env,
                info,
                &token_pair.quote_token,
                amount,
            )?;
        }
        Ok(())
    }

    fn validate_native_token_payment(
        info: &MessageInfo,
        denom: &str,
        required_amount: Uint128,
    ) -> StdResult<()> {
        // Find the coin with matching denom in the sent funds
        let sent_amount = info
            .funds
            .iter()
            .find(|coin| coin.denom == denom)
            .map(|coin| coin.amount)
            .unwrap_or_default();

        // Check if sent amount matches required amount
        if sent_amount < required_amount {
            return Err(StdError::generic_err(format!(
                "Insufficient native token sent. Required: {}, Sent: {}",
                required_amount, sent_amount
            )));
        }

        // Check if excess amount was sent
        if sent_amount > required_amount {
            return Err(StdError::generic_err(format!(
                "Excess native token sent. Required: {}, Sent: {}",
                required_amount, sent_amount
            )));
        }

        Ok(())
    }

    fn validate_cw20_token_payment(
        deps: &Deps,
        env: &Env,
        info: &MessageInfo,
        token_address: &str,
        required_amount: Uint128,
    ) -> StdResult<()> {
        // Query token balance
        let balance: cw20::BalanceResponse = deps.querier.query_wasm_smart(
            token_address,
            &cw20::Cw20QueryMsg::Balance {
                address: info.sender.to_string(),
            },
        )?;

        // Check if user has sufficient balance
        if balance.balance < required_amount {
            return Err(StdError::generic_err(format!(
                "Insufficient CW20 token balance. Required: {}, Balance: {}",
                required_amount, balance.balance
            )));
        }

        // Query allowance
        let allowance: cw20::AllowanceResponse = deps.querier.query_wasm_smart(
            token_address,
            &cw20::Cw20QueryMsg::Allowance {
                owner: info.sender.to_string(),
                spender: env.contract.address.to_string(),
            },
        )?;

        // Check if contract has sufficient allowance
        if allowance.allowance < required_amount {
            return Err(StdError::generic_err(format!(
                "Insufficient CW20 token allowance. Required: {}, Allowance: {}",
                required_amount, allowance.allowance
            )));
        }

        Ok(())
    }

    fn create_order(
        storage: &mut dyn Storage,
        buy_order: &mut Order,
        sell_order: &mut Order,
        trade_id: u64,
        token_pair: &TokenPair,
        config: &Config,
        env: &Env,
    ) -> StdResult<Response> {
        // Calculate the trade amount (minimum of remaining amounts)
        let trade_amount = std::cmp::min(buy_order.remaining_amount, sell_order.remaining_amount);

        // Use the sell order price for the trade (price-time priority)
        let trade_price = sell_order.price;
        let total_price = trade_amount * trade_price;

        // Calculate fees - maker gets a lower fee rate than taker
        let maker_fee_rate = config.maker_fee.to_uint_ceil();
        let taker_fee_rate = config.taker_fee.to_uint_ceil();

        // Determine maker and taker based on order timestamps
        let (maker_fee_amount, taker_fee_amount) = if buy_order.timestamp < sell_order.timestamp {
            let maker_fee = (total_price * maker_fee_rate) / Uint128::from(10000u64);
            let taker_fee = (total_price * taker_fee_rate) / Uint128::from(10000u64);
            (maker_fee, taker_fee)
        } else {
            let maker_fee = (total_price * maker_fee_rate) / Uint128::from(10000u64);
            let taker_fee = (total_price * taker_fee_rate) / Uint128::from(10000u64);
            (taker_fee, maker_fee)
        };

        // Update order amounts
        buy_order.remaining_amount -= trade_amount;
        buy_order.filled_amount += trade_amount;
        if buy_order.remaining_amount.is_zero() {
            buy_order.status = OrderStatus::Filled;
        }

        sell_order.remaining_amount -= trade_amount;
        sell_order.filled_amount += trade_amount;
        if sell_order.remaining_amount.is_zero() {
            sell_order.status = OrderStatus::Filled;
        }

        // Save updated orders to storage
        USER_ORDERS.save(storage, (buy_order.owner.clone(), buy_order.id), buy_order)?;
        USER_ORDERS.save(
            storage,
            (sell_order.owner.clone(), sell_order.id),
            sell_order,
        )?;

        // Create response with trade event
        Ok(Response::new()
            .add_attribute("event_type", "trade")
            .add_attribute("trade_id", trade_id.to_string())
            .add_attribute("pair_id", buy_order.pair_id.clone())
            .add_attribute("buy_order_id", buy_order.id.to_string())
            .add_attribute("sell_order_id", sell_order.id.to_string())
            .add_attribute("price", trade_price.to_string())
            .add_attribute("amount", trade_amount.to_string())
            .add_attribute("total", total_price.to_string())
            .add_attribute("maker_fee", maker_fee_amount.to_string())
            .add_attribute("taker_fee", taker_fee_amount.to_string())
            .add_attribute("base_token", token_pair.base_token.clone())
            .add_attribute("quote_token", token_pair.quote_token.clone())
            .add_attribute("timestamp", env.block.time.seconds().to_string()))
    }

    fn execute_cw20_transfer(
        token_address: &str,
        from: &Addr,
        to: &Addr,
        amount: Uint128,
    ) -> StdResult<CosmosMsg> {
        Ok(WasmMsg::Execute {
            contract_addr: token_address.to_string(),
            msg: to_json_binary(&cw20::Cw20ExecuteMsg::TransferFrom {
                owner: from.to_string(),
                recipient: to.to_string(),
                amount,
            })?,
            funds: vec![],
        }
        .into())
    }

    // Helper function to handle native token transfers
    fn execute_native_transfer(denom: &str, to: &Addr, amount: Uint128) -> StdResult<CosmosMsg> {
        Ok(BankMsg::Send {
            to_address: to.to_string(),
            amount: vec![Coin {
                denom: denom.to_string(),
                amount,
            }],
        }
        .into())
    }

    fn clean_up_order_book(order_book: &mut OrderBook) {
        // Remove filled buy orders
        order_book.buy_orders.retain(|_, orders| {
            orders.retain(|order| !order.remaining_amount.is_zero());
            !orders.is_empty()
        });

        // Remove filled sell orders
        order_book.sell_orders.retain(|_, orders| {
            orders.retain(|order| !order.remaining_amount.is_zero());
            !orders.is_empty()
        });
    }

    fn match_limit_orders(
        storage: &mut dyn Storage,
        info: &MessageInfo,
        env: &Env,
        pair_id: String,
        amount: Uint128,
        order_type: &OrderType,
        min_return: Uint128,
    ) -> StdResult<(Uint128, Uint128, Response)> {
        let mut response = Response::new();
        let mut order_book = ORDER_BOOKS.load(storage, pair_id.clone())?;
        let config = CONFIG.load(storage)?;
        let token_pair = TOKEN_PAIRS.load(storage, pair_id.clone())?;
        let mut next_trade_id = NEXT_TRADE_ID.load(storage)?;

        let mut matched_amount = Uint128::zero();
        let mut remaining_amount = amount;
        let mut total_return_amount = Uint128::zero();

        let orders_to_match = match order_type {
            OrderType::Buy => &mut order_book.sell_orders,
            OrderType::Sell => &mut order_book.buy_orders,
        };

        // Sort orders by price (best price first)
        let mut price_levels: Vec<_> = orders_to_match.keys().cloned().collect();
        match order_type {
            OrderType::Buy => price_levels.sort(), // Ascending for buys (lowest price first)
            OrderType::Sell => price_levels.sort_by(|a, b| b.cmp(a)), // Descending for sells (highest price first)
        }

        for price_level in price_levels {
            if remaining_amount.is_zero() {
                break;
            }

            let orders = orders_to_match.get_mut(&price_level).unwrap();
            let mut i = 0;
            while i < orders.len() && !remaining_amount.is_zero() {
                let order = &mut orders[i];

                // Skip already filled orders or orders with different price
                if order.remaining_amount.is_zero() || order.price != Uint128::from(price_level) {
                    i += 1;
                    continue;
                }

                let match_amount = std::cmp::min(remaining_amount, order.remaining_amount);
                let trade_price = Uint128::from(price_level);
                let trade_return_amount = match_amount * trade_price;

                // Calculate fees
                let total_price = match_amount * trade_price;
                let maker_fee = (Decimal::new(total_price) * config.maker_fee)
                    .checked_div(Decimal::percent(100))
                    .unwrap();
                let taker_fee = (Decimal::new(total_price) * config.taker_fee)
                    .checked_div(Decimal::percent(100))
                    .unwrap();

                if !match_amount.is_zero() {
                    // Determine buyer and seller based on order type
                    let (buyer, seller, buy_order_id, sell_order_id) = match order_type {
                        OrderType::Buy => {
                            (order.owner.clone(), &info.sender, order.id, next_trade_id)
                        }
                        OrderType::Sell => (
                            info.sender.clone(),
                            &order.owner.clone(),
                            next_trade_id,
                            order.id,
                        ),
                    };

                    // Execute the trade
                    execute_trade(
                        storage,
                        env,
                        &buyer,
                        &seller,
                        pair_id.clone(),
                        buy_order_id,
                        sell_order_id,
                        match_amount,
                        trade_price,
                        maker_fee.to_uint_ceil(),
                        taker_fee.to_uint_ceil(),
                    )?;

                    // Create trade event attributes
                    let trade_attrs = vec![
                        ("event_type", "trade".to_string()),
                        ("trade_id", next_trade_id.to_string()),
                        ("pair_id", pair_id.clone()),
                        (
                            "buy_order_id",
                            if order_type == &OrderType::Buy {
                                "market_order".to_string()
                            } else {
                                order.id.to_string()
                            },
                        ),
                        (
                            "sell_order_id",
                            if order_type == &OrderType::Sell {
                                "market_order".to_string()
                            } else {
                                order.id.to_string()
                            },
                        ),
                        ("price", trade_price.to_string()),
                        ("amount", match_amount.to_string()),
                        ("total", total_price.to_string()),
                        ("maker_fee", maker_fee.to_string()),
                        ("taker_fee", taker_fee.to_string()),
                        ("base_token", token_pair.base_token.clone()),
                        ("quote_token", token_pair.quote_token.clone()),
                        ("timestamp", env.block.time.seconds().to_string()),
                    ]
                    .into_iter()
                    .map(|(k, v)| attr(k, v))
                    .collect::<Vec<_>>();

                    response = response.add_attributes(trade_attrs);

                    // Update amounts
                    matched_amount += match_amount;
                    remaining_amount -= match_amount;
                    order.remaining_amount -= match_amount;
                    order.filled_amount += match_amount;
                    total_return_amount += trade_return_amount;

                    next_trade_id += 1;
                }

                // Check if the total return amount meets the minimum return requirement
                if total_return_amount >= min_return {
                    break;
                }

                i += 1;
            }

            // Remove filled orders
            orders.retain(|order| !order.remaining_amount.is_zero());
        }

        // Clean up empty price levels and save updated state
        clean_up_order_book(&mut order_book);

        ORDER_BOOKS.save(storage, pair_id, &order_book)?;
        NEXT_TRADE_ID.save(storage, &next_trade_id)?;

        // Add final amounts to response
        response = response.add_attributes(vec![
            attr("matched_amount", matched_amount),
            attr("remaining_amount", remaining_amount),
        ]);

        Ok((matched_amount, remaining_amount, response))
    }

    // Function to execute bonding curve swap
    fn execute_bonding_curve_swap(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        pair_id: String,
        token_address: String,
        amount: Uint128,
        min_return: Uint128,
        order_type: OrderType,
    ) -> StdResult<Response> {
        let config = CONFIG.load(deps.storage)?;
        if !config.enabled {
            return Err(StdError::generic_err("Trading is currently disabled"));
        }

        let mut pool = POOLS.load(deps.storage, token_address.clone())?;
        let token_pair = TOKEN_PAIRS.load(deps.storage, pair_id.clone())?;
        if !token_pair.enabled {
            return Err(StdError::generic_err("Trading pair is disabled"));
        }

        if !pool.enabled {
            return Err(StdError::generic_err("Pool is disabled"));
        }

        let price = calculate_exponential_price(
            deps.storage,
            token_address.clone(),
            pool.token_sold,
            amount,
            matches!(order_type, OrderType::Buy),
        )?;

        // Calculate the swap amounts based on the price
        let (base_amount, quote_amount, messages) = match order_type {
            OrderType::Buy => {
                // When buying quote tokens with base tokens
                validate_native_or_cw20_payment(
                    deps.as_ref(),
                    &info,
                    &env,
                    &token_pair.base_token,
                    amount,
                    true,
                )?;

                let tokens_to_receive = amount * price / Uint128::new(1_000_000); // Using 6 decimals for price

                // Check if pool has enough quote tokens
                if tokens_to_receive > Uint128::from(config.bonding_curve_supply) - pool.token_sold
                {
                    return Err(StdError::generic_err("Insufficient liquidity in pool"));
                }

                if tokens_to_receive < min_return {
                    return Err(StdError::generic_err(format!(
                        "Slippage tolerance exceeded. Expected: {}, Minimum: {}",
                        tokens_to_receive, min_return
                    )));
                }

                // Update pool reserves
                pool.total_reserve_token += amount;
                pool.token_sold += tokens_to_receive;

                // Prepare transfer messages
                let mut msgs = vec![];

                // Transfer quote tokens from pool to sender
                msgs.push(execute_cw20_transfer(
                    &token_pair.quote_token,
                    &env.contract.address,
                    &info.sender,
                    tokens_to_receive,
                )?);

                (amount, tokens_to_receive, msgs)
            }
            OrderType::Sell => {
                // When selling quote tokens for base tokens
                validate_native_or_cw20_payment(
                    deps.as_ref(),
                    &info,
                    &env,
                    &token_pair.quote_token,
                    amount,
                    false,
                )?;

                let base_to_receive = amount * price / Uint128::new(1_000_000);

                // Check if pool has enough base tokens
                if base_to_receive > pool.token_sold {
                    return Err(StdError::generic_err("Insufficient liquidity in pool"));
                }

                if base_to_receive < min_return {
                    return Err(StdError::generic_err(format!(
                        "Slippage tolerance exceeded. Expected: {}, Minimum: {}",
                        base_to_receive, min_return
                    )));
                }

                // Update pool reserves
                pool.token_sold -= amount;
                pool.total_reserve_token -= base_to_receive;

                // Prepare transfer messages
                let mut msgs = vec![];

                // Transfer CW20 tokens from sender to pool
                msgs.push(execute_cw20_transfer(
                    &token_pair.quote_token,
                    &info.sender,
                    &env.contract.address,
                    amount,
                )?);

                // Transfer base tokens from pool to sender
                msgs.push(execute_native_transfer(
                    &token_pair.base_token,
                    &info.sender,
                    base_to_receive,
                )?);

                (base_to_receive, amount, msgs)
            }
        };

        // Update pool state
        pool.last_price = price;
        pool.total_trades += Uint128::new(1);
        POOLS.save(deps.storage, token_address.clone(), &pool)?;

        Ok(Response::new().add_messages(messages).add_attributes(vec![
            attr("action", "bonding_curve_swap"),
            attr("pair_id", pair_id),
            attr("order_type", format!("{:?}", order_type)),
            attr("base_amount", base_amount),
            attr("quote_amount", quote_amount),
            attr("price", price.to_string()),
        ]))
    }

    fn validate_native_or_cw20_payment(
        deps: Deps,
        info: &MessageInfo,
        env: &Env,
        token: &str,
        required_amount: Uint128,
        is_native: bool,
    ) -> StdResult<()> {
        if is_native {
            validate_native_token_payment(info, token, required_amount)
        } else {
            validate_cw20_token_payment(&deps, env, info, token, required_amount)
        }
    }

    // Helper function to execute trade and update user history
    fn execute_trade(
        storage: &mut dyn Storage,
        env: &Env,
        buyer: &Addr,
        seller: &Addr,
        pair_id: String,
        buy_order_id: u64,
        sell_order_id: u64,
        amount: Uint128,
        price: Uint128,
        maker_fee: Uint128,
        taker_fee: Uint128,
    ) -> StdResult<()> {
        // Create trade record
        let trade = Trade {
            id: NEXT_TRADE_ID.load(storage)?,
            pair_id,
            buy_order_id,
            sell_order_id,
            buyer: buyer.clone(),
            seller: seller.clone(),
            token_amount: amount,
            price,
            timestamp: env.block.time.seconds() as u64,
            total_price: amount * price,
            maker_fee_amount: maker_fee,
            taker_fee_amount: taker_fee,
        };

        // Update user trade counts and histories
        for user in [buyer, seller] {
            let count = USER_TRADE_COUNT.load(storage, user.clone()).unwrap_or(0);
            if count >= MAX_TRADES_PER_USER as u64 {
                // Remove oldest trade
                USER_TRADES.remove(storage, (user.clone(), count - MAX_TRADES_PER_USER as u64));
            }
            USER_TRADES.save(storage, (user.clone(), count), &trade)?;
            USER_TRADE_COUNT.save(storage, user.clone(), &(count + 1))?;
        }

        TRADES.save(storage, trade.id, &trade)?;
        NEXT_TRADE_ID.save(storage, &(trade.id + 1u64))?;

        Ok(())
    }

    fn calculate_exponential_price(
        storage: &dyn Storage,
        token_address: String,
        current_supply: Uint128,
        amount: Uint128,
        is_buy: bool,
    ) -> StdResult<Uint128> {
        let config = CONFIG.load(storage)?;
        let pool = POOLS.load(storage, token_address.clone())?;
        let token_info = TOKEN_INFO.load(storage, token_address)?;

        let base_price = Decimal::from_ratio(BASE_PRICE, Uint128::new(1_000_000));
        let slope = Decimal::from_ratio(pool.curve_slope, Uint128::new(1_000_000));

        if pool.token_sold + amount >= Uint128::from(config.bonding_curve_supply) {
            return Err(StdError::generic_err(
                "Supply exceeds maximum limit for pricing",
            ));
        }

        let (lower_bound, upper_bound) = if is_buy {
            (current_supply, current_supply + amount)
        } else {
            (current_supply - amount, current_supply)
        };

        let lower_dec = Decimal::from_ratio(lower_bound, Uint128::new(1));
        let upper_dec = Decimal::from_ratio(upper_bound, Uint128::new(1));

        let alpha = Decimal::from_str("0.1").unwrap();

        let exp_upper = calculate_ema_exp(slope * upper_dec, alpha)?;
        let exp_lower = calculate_ema_exp(slope * lower_dec, alpha)?;

        let avg_price = base_price * (exp_upper - exp_lower)
            / (slope * Decimal::from_ratio(amount, Uint128::new(1)));

        let price_uint = Uint128::new(
            (avg_price * Decimal::from_ratio(10_u128.pow(token_info.decimals as u32), 1u128))
                .to_uint_floor()
                .try_into()
                .map_err(|_| StdError::generic_err("Price overflow"))?,
        );

        Ok(price_uint)
    }

    fn calculate_ema_exp(x: Decimal, alpha: Decimal) -> StdResult<Decimal> {
        let mut ema = Decimal::one();
        let steps = 100;
        for _ in 0..steps {
            ema = alpha * x + (Decimal::one() - alpha) * ema;
        }
        Ok(ema)
    }

    #[cfg(test)]
    mod tests {
        use std::collections::BTreeMap;

        use super::*;
        use cosmwasm_std::testing::MockQuerier;
        use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
        use cosmwasm_std::{
            coins, from_binary, from_json, Addr, Coin, CosmosMsg, Decimal, Querier, SystemError,
            Uint128, WasmMsg,
        };
        use cosmwasm_std::{ContractResult, Env, QueryRequest, StdError, SystemResult, WasmQuery};
        use cosmwasm_std::{Response, SubMsg};
        use cw20::{AllowanceResponse, BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg, Expiration};
        use token_factory::contract::execute::generate_token_address;
        use token_factory::state::Cw20Coin;

        #[test]
        fn test_proper_instantiate() {
            let mut deps = mock_dependencies();

            let msg = InstantiateMsg {
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee_rate: Decimal::percent(1), // 1%
                quote_token_total_supply: Uint128::from(100_000_000_000u128),
                bonding_curve_supply: Uint128::from(80_000_000_000u128),
                lp_supply: Uint128::from(20_000_000_000u128),
                maker_fee: Uint128::from(1u128),
                taker_fee: Uint128::from(1u128),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "uhuahua".to_string(),
            };

            let info = message_info(&Addr::unchecked("creator"), &[]);

            // Instantiate the contract
            let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

            // Check the response
            assert_eq!(res.attributes.len(), 12);
            assert_eq!(res.attributes[0].key, "action");
            assert_eq!(res.attributes[0].value, "instantiate");
            assert_eq!(res.attributes[1].key, "owner");
            assert_eq!(res.attributes[1].value, info.sender.to_string());
            assert_eq!(res.attributes[2].key, "token_factory");
            assert_eq!(res.attributes[2].value, msg.token_factory.to_string());
            assert_eq!(res.attributes[3].key, "trading_fee_rate");
            assert_eq!(res.attributes[3].value, msg.trading_fee_rate.to_string());
            assert_eq!(res.attributes[4].key, "fee_collector");
            assert_eq!(res.attributes[4].value, msg.fee_collector.to_string());
            assert_eq!(res.attributes[5].key, "quote_token_total_supply");
            assert_eq!(
                res.attributes[5].value,
                msg.quote_token_total_supply.to_string()
            );
            assert_eq!(res.attributes[6].key, "bonding_curve_supply");
            assert_eq!(
                res.attributes[6].value,
                msg.bonding_curve_supply.to_string()
            );
            assert_eq!(res.attributes[7].key, "lp_supply");
            assert_eq!(res.attributes[7].value, msg.lp_supply.to_string());
            assert_eq!(res.attributes[8].key, "maker_fee");
            assert_eq!(res.attributes[8].value, msg.maker_fee.to_string());
            assert_eq!(res.attributes[9].key, "taker_fee");
            assert_eq!(res.attributes[9].value, msg.taker_fee.to_string());
            assert_eq!(res.attributes[10].key, "secondary_amm_address");
            assert_eq!(
                res.attributes[10].value,
                msg.secondary_amm_address.to_string()
            );
            assert_eq!(res.attributes[11].key, "base_token_denom");
            assert_eq!(res.attributes[11].value, msg.base_token_denom.to_string());

            // Verify state was set correctly
            let config = CONFIG.load(&deps.storage).unwrap();
            assert_eq!(config.owner, info.sender);
            assert_eq!(config.token_factory, msg.token_factory);
            assert_eq!(config.fee_collector, msg.fee_collector);
            assert_eq!(config.trading_fee, msg.trading_fee_rate);
            assert_eq!(config.enabled, true);
            assert_eq!(
                config.quote_token_total_supply,
                msg.quote_token_total_supply.into()
            );
            assert_eq!(config.bonding_curve_supply, msg.bonding_curve_supply.into());
            assert_eq!(config.lp_supply, msg.lp_supply.into());
            assert_eq!(
                config.maker_fee,
                Decimal::from_ratio(msg.maker_fee, Uint128::one())
            );
            assert_eq!(
                config.taker_fee,
                Decimal::from_ratio(msg.taker_fee, Uint128::one())
            );
            assert_eq!(config.secondary_amm_address, msg.secondary_amm_address);
            assert_eq!(config.base_token_denom, msg.base_token_denom);

            let next_order_id = NEXT_ORDER_ID.load(&deps.storage).unwrap();
            assert_eq!(next_order_id, 0u64);

            let next_trade_id = NEXT_TRADE_ID.load(&deps.storage).unwrap();
            assert_eq!(next_trade_id, 0u64);
        }

        #[test]
        fn test_execute_create_token_happy_case() {
            let mut deps = mock_dependencies();
            let env = mock_env();

            // Setup initial configuration
            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "base_token_denom".to_string(),
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();

            let info = message_info(&Addr::unchecked("creator"), &[]);

            // Define parameters for creating the token
            let name = "Test Token".to_string();
            let symbol = "TST".to_string();
            let decimals = 8;
            let max_price_impact = Uint128::from(100u128);
            let curve_slope = Uint128::from(1u128);

            // Execute the create_token function
            let res = execute_create_token(
                deps.as_mut(),
                mock_env(),
                info.clone(),
                name.clone(),
                symbol.clone(),
                decimals,
                max_price_impact,
                curve_slope,
            )
            .unwrap();

            // Check the response
            assert_eq!(res.attributes.len(), 6);
            assert_eq!(res.attributes[0].key, "action");
            assert_eq!(res.attributes[0].value, "create_token");
            assert_eq!(res.attributes[1].key, "name");
            assert_eq!(res.attributes[1].value, name);
            assert_eq!(res.attributes[2].key, "symbol");
            assert_eq!(res.attributes[2].value, symbol);
            assert_eq!(res.attributes[3].key, "decimals");
            assert_eq!(res.attributes[3].value, decimals.to_string());
            assert_eq!(res.attributes[4].key, "max_price_impact");
            assert_eq!(res.attributes[4].value, max_price_impact.to_string());
            assert_eq!(res.attributes[5].key, "curve_slope");
            assert_eq!(res.attributes[5].value, curve_slope.to_string());

            // Check that the response includes the correct message and attributes
            let msg = res.messages.get(0).expect("no message");
            assert_eq!(
                msg.msg,
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: "token_factory_addr".to_string(),
                    msg: to_json_binary(&token_factory::msg::ExecuteMsg::CreateToken {
                        name: name.clone(),
                        symbol: symbol.clone(),
                        decimals,
                        initial_balances: vec![Cw20Coin {
                            address: env.contract.address.to_string(),
                            amount: Uint128::from(
                                100_000_000_000u128 * 10u128.pow(decimals as u32)
                            ),
                        }],
                    })
                    .unwrap(),
                    funds: vec![],
                })
            );

            // Verify that the token information was saved correctly
            let token_address = generate_token_address(&name, &symbol);
            let token_info = TOKEN_INFO
                .load(&deps.storage, token_address.to_string())
                .unwrap();
            assert_eq!(token_info.name, name);
            assert_eq!(token_info.symbol, symbol);
            assert_eq!(token_info.decimals, decimals);
            assert_eq!(
                token_info.total_supply,
                Uint128::from(100_000_000_000u128 * 10u128.pow(decimals as u32))
            );
            assert_eq!(token_info.initial_price, Uint128::from(BASE_PRICE));
            assert_eq!(token_info.max_price_impact, max_price_impact);
            assert_eq!(token_info.graduated, false);

            // Verify that the token pair information was saved correctly
            let pair_id = format!("{}/{}", symbol, &config.base_token_denom[1..]);
            let token_pair = TOKEN_PAIRS.load(&deps.storage, pair_id.clone()).unwrap();
            assert_eq!(token_pair.base_token, config.base_token_denom);
            assert_eq!(token_pair.quote_token, token_address.to_string());
            assert_eq!(token_pair.base_decimals, 6);
            assert_eq!(token_pair.quote_decimals, decimals);
            assert_eq!(token_pair.enabled, true);

            // Verify that the pool information was saved correctly
            let pool = POOLS
                .load(&deps.storage, token_address.to_string())
                .unwrap();
            assert_eq!(pool.token_address, token_address);
            assert_eq!(pool.total_reserve_token, Uint128::zero());
            assert_eq!(pool.token_sold, Uint128::zero());
            assert_eq!(pool.total_volume, Uint128::zero());
            assert_eq!(pool.total_fees_collected, Uint128::zero());
            assert_eq!(pool.curve_slope, curve_slope);
            assert_eq!(pool.pair_id, pair_id);
            assert_eq!(pool.total_trades, Uint128::zero());
            assert_eq!(pool.last_price, Uint128::from(BASE_PRICE));
            assert_eq!(pool.enabled, true);
        }

        #[test]
        fn test_create_order_happy_case() {
            let mut deps = mock_dependencies();
            let env = mock_env();

            // Create initial buy and sell orders
            let mut buy_order = Order {
                id: 1,
                owner: Addr::unchecked("buyer"),
                pair_id: "pair_id".to_string(),
                token_amount: Uint128::from(100u128),
                price: Uint128::from(10u128),
                timestamp: env.block.time.seconds() as u64,
                status: OrderStatus::Active,
                filled_amount: Uint128::zero(),
                remaining_amount: Uint128::from(100u128),
                order_type: OrderType::Buy,
                created_at: env.block.height,
            };

            let mut sell_order = Order {
                id: 2,
                owner: Addr::unchecked("seller"),
                pair_id: "pair_id".to_string(),
                token_amount: Uint128::from(100u128),
                price: Uint128::from(10u128),
                timestamp: env.block.time.seconds() as u64,
                status: OrderStatus::Active,
                filled_amount: Uint128::zero(),
                remaining_amount: Uint128::from(100u128),
                order_type: OrderType::Sell,
                created_at: env.block.height,
            };

            // Define token pair and config
            let token_pair = TokenPair {
                base_token: "base_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
                enabled: true,
            };

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "base_token_denom".to_string(),
            };

            // Execute the create_trade function
            let trade_id = 1;
            let res = create_order(
                &mut deps.storage,
                &mut buy_order,
                &mut sell_order,
                trade_id,
                &token_pair,
                &config,
                &env,
            )
            .unwrap();

            // Check the response
            assert_eq!(res.attributes.len(), 13);
            assert_eq!(res.attributes[0].key, "event_type");
            assert_eq!(res.attributes[0].value, "trade");
            assert_eq!(res.attributes[1].key, "trade_id");
            assert_eq!(res.attributes[1].value, trade_id.to_string());
            assert_eq!(res.attributes[2].key, "pair_id");
            assert_eq!(res.attributes[2].value, buy_order.pair_id.clone());
            assert_eq!(res.attributes[3].key, "buy_order_id");
            assert_eq!(res.attributes[3].value, buy_order.id.to_string());
            assert_eq!(res.attributes[4].key, "sell_order_id");
            assert_eq!(res.attributes[4].value, sell_order.id.to_string());
            assert_eq!(res.attributes[5].key, "price");
            assert_eq!(res.attributes[5].value, sell_order.price.to_string());
            assert_eq!(res.attributes[6].key, "amount");
            assert_eq!(res.attributes[6].value, buy_order.token_amount.to_string());
            assert_eq!(res.attributes[7].key, "total");
            assert_eq!(
                res.attributes[7].value,
                (buy_order.token_amount * sell_order.price).to_string()
            );
            assert_eq!(res.attributes[8].key, "maker_fee");
            assert_eq!(
                res.attributes[8].value,
                ((buy_order.token_amount * sell_order.price * config.maker_fee.to_uint_ceil())
                    / Uint128::from(10000u64))
                .to_string()
            );
            assert_eq!(res.attributes[9].key, "taker_fee");
            assert_eq!(
                res.attributes[9].value,
                ((buy_order.token_amount * sell_order.price * config.taker_fee.to_uint_ceil())
                    / Uint128::from(10000u64))
                .to_string()
            );
            assert_eq!(res.attributes[10].key, "base_token");
            assert_eq!(res.attributes[10].value, token_pair.base_token.clone());
            assert_eq!(res.attributes[11].key, "quote_token");
            assert_eq!(res.attributes[11].value, token_pair.quote_token.clone());
            assert_eq!(res.attributes[12].key, "timestamp");
            assert_eq!(
                res.attributes[12].value,
                env.block.time.seconds().to_string()
            );

            // Check updated order amounts
            assert_eq!(buy_order.filled_amount, buy_order.token_amount);
            assert_eq!(buy_order.remaining_amount, Uint128::zero());
            assert_eq!(buy_order.status, OrderStatus::Filled);
            assert_eq!(sell_order.filled_amount, sell_order.token_amount);
            assert_eq!(sell_order.remaining_amount, Uint128::zero());
            assert_eq!(sell_order.status, OrderStatus::Filled);

            // Verify orders were saved correctly in storage
            let saved_buy_order = USER_ORDERS
                .load(&deps.storage, (buy_order.owner.clone(), buy_order.id))
                .unwrap();
            assert_eq!(saved_buy_order, buy_order);

            let saved_sell_order = USER_ORDERS
                .load(&deps.storage, (sell_order.owner.clone(), sell_order.id))
                .unwrap();
            assert_eq!(saved_sell_order, sell_order);
        }

        #[test]
        fn test_execute_place_limit_order_buy() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let info = message_info(
                &Addr::unchecked("buyer"),
                &[Coin {
                    denom: "base_token".to_string(),
                    amount: Uint128::from(1000u128),
                }],
            );

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token".to_string(),
            };

            let token_pair = TokenPair {
                base_token: "base_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
                enabled: true,
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();
            TOKEN_PAIRS
                .save(deps.as_mut().storage, "pair_id".to_string(), &token_pair)
                .unwrap();

            ORDER_BOOKS
                .save(
                    deps.as_mut().storage,
                    "pair_id".to_string(),
                    &OrderBook {
                        pair_id: "pair_id".to_string(),
                        buy_orders: BTreeMap::new(),
                        sell_orders: BTreeMap::new(),
                    },
                )
                .unwrap();

            NEXT_ORDER_ID.save(deps.as_mut().storage, &0u64).unwrap();
            NEXT_TRADE_ID.save(deps.as_mut().storage, &0u64).unwrap(); // Initialize NEXT_TRADE_ID

            let amount = Uint128::from(100u128);
            let price = Uint128::from(10u128);
            let is_buy = true;

            // Execute the place limit order function
            let res = execute_place_limit_order(
                deps.as_mut(),
                env,
                info.clone(),
                "pair_id".to_string(),
                amount,
                price,
                is_buy,
            )
            .unwrap();

            // Verify response attributes
            assert_eq!(res.attributes.len(), 6);
            assert_eq!(res.attributes[0].key, "action");
            assert_eq!(res.attributes[0].value, "place_limit_order");
            assert_eq!(res.attributes[1].key, "order_id");
            assert_eq!(res.attributes[1].value, "0");
            assert_eq!(res.attributes[2].key, "pair_id");
            assert_eq!(res.attributes[2].value, "pair_id");
            assert_eq!(res.attributes[3].key, "is_buy");
            assert_eq!(res.attributes[3].value, "true");
            assert_eq!(res.attributes[4].key, "amount");
            assert_eq!(res.attributes[4].value, amount.to_string());
            assert_eq!(res.attributes[5].key, "price");
            assert_eq!(res.attributes[5].value, price.to_string());

            // Verify the order was added to the order book
            let order_book = ORDER_BOOKS
                .load(&deps.storage, "pair_id".to_string())
                .unwrap();
            assert_eq!(order_book.buy_orders.len(), 1);
            assert_eq!(order_book.buy_orders.get(&price.u128()).unwrap().len(), 1);

            // Verify the order was saved in user orders
            let order = USER_ORDERS
                .load(&deps.storage, (info.sender.clone(), 0))
                .unwrap();
            assert_eq!(order.owner, Addr::unchecked("buyer"));
            assert_eq!(order.pair_id, "pair_id");
            assert_eq!(order.token_amount, amount);
            assert_eq!(order.price, price);
            assert_eq!(order.remaining_amount, amount);
            assert_eq!(order.order_type, OrderType::Buy);
        }

        #[test]
        fn test_execute_place_limit_order_sell() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let info = message_info(&Addr::unchecked("seller"), &[]);

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token_denom".to_string(),
            };

            let token_pair = TokenPair {
                base_token: "base_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
                enabled: true,
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();
            TOKEN_PAIRS
                .save(deps.as_mut().storage, "pair_id".to_string(), &token_pair)
                .unwrap();

            ORDER_BOOKS
                .save(
                    deps.as_mut().storage,
                    "pair_id".to_string(),
                    &OrderBook {
                        pair_id: "pair_id".to_string(),
                        buy_orders: BTreeMap::new(),
                        sell_orders: BTreeMap::new(),
                    },
                )
                .unwrap();

            NEXT_TRADE_ID.save(deps.as_mut().storage, &0u64).unwrap();
            NEXT_ORDER_ID.save(deps.as_mut().storage, &0u64).unwrap();

            let amount = Uint128::from(100u128);
            let price = Uint128::from(10u128);
            let is_buy = false;

            // Mock balance and allowance queries for CW20 tokens
            let sell_info = message_info(&Addr::unchecked("seller"), &[]);
            let cw20_token_address = String::from("quote_token");

            // Clone necessary variables to avoid moving them into the closure
            let cw20_token_address_clone = cw20_token_address.clone();
            let sell_info_sender_clone = sell_info.sender.clone();

            // Mock the balance and allowance queries
            deps.querier.update_wasm(move |query| {
                let cw20_token_address = cw20_token_address_clone.clone();
                let sell_info_sender = sell_info_sender_clone.clone();
                let env = mock_env();

                match query {
                    WasmQuery::Smart { contract_addr, msg } => {
                        if contract_addr == &cw20_token_address.clone() {
                            if let Ok(cw20::Cw20QueryMsg::Balance { address }) = from_json(&msg) {
                                if address == sell_info_sender.into_string() {
                                    return SystemResult::Ok(ContractResult::Ok(
                                        to_json_binary(&BalanceResponse {
                                            balance: Uint128::from(1000u128),
                                        })
                                        .unwrap(),
                                    ));
                                }
                            } else if let Ok(cw20::Cw20QueryMsg::Allowance { owner, spender }) =
                                from_json(&msg)
                            {
                                if owner == sell_info_sender.into_string()
                                    && spender == env.contract.address.to_string()
                                {
                                    return SystemResult::Ok(ContractResult::Ok(
                                        to_json_binary(&AllowanceResponse {
                                            allowance: Uint128::from(1000u128),
                                            expires: Expiration::Never {},
                                        })
                                        .unwrap(),
                                    ));
                                }
                            }
                        }
                        SystemResult::Err(SystemError::UnsupportedRequest {
                            kind: "".to_string(),
                        })
                    }
                    _ => SystemResult::Err(SystemError::UnsupportedRequest {
                        kind: "".to_string(),
                    }),
                }
            });

            // Execute the place limit order function
            let res = execute_place_limit_order(
                deps.as_mut(),
                env,
                info.clone(),
                "pair_id".to_string(),
                amount,
                price,
                is_buy,
            )
            .unwrap();

            // Verify response attributes
            assert_eq!(res.attributes.len(), 6);
            assert_eq!(res.attributes[0].key, "action");
            assert_eq!(res.attributes[0].value, "place_limit_order");
            assert_eq!(res.attributes[1].key, "order_id");
            assert_eq!(res.attributes[1].value, "0");
            assert_eq!(res.attributes[2].key, "pair_id");
            assert_eq!(res.attributes[2].value, "pair_id");
            assert_eq!(res.attributes[3].key, "is_buy");
            assert_eq!(res.attributes[3].value, "false");
            assert_eq!(res.attributes[4].key, "amount");
            assert_eq!(res.attributes[4].value, amount.to_string());
            assert_eq!(res.attributes[5].key, "price");
            assert_eq!(res.attributes[5].value, price.to_string());

            // Verify the order was added to the order book
            let order_book = ORDER_BOOKS
                .load(&deps.storage, "pair_id".to_string())
                .unwrap();
            assert_eq!(order_book.sell_orders.len(), 1);
            assert_eq!(order_book.sell_orders.get(&price.u128()).unwrap().len(), 1);

            // Verify the order was saved in user orders
            let order = USER_ORDERS
                .load(&deps.storage, (info.sender.clone(), 0))
                .unwrap();
            assert_eq!(order.owner, Addr::unchecked("seller"));
            assert_eq!(order.pair_id, "pair_id");
            assert_eq!(order.token_amount, amount);
            assert_eq!(order.price, price);
            assert_eq!(order.remaining_amount, amount);
            assert_eq!(order.order_type, OrderType::Sell);
        }

        #[test]
        fn test_execute_place_limit_order_matching_orders() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let buyer_info = message_info(
                &Addr::unchecked("buyer"),
                &[Coin {
                    denom: "base_token".to_string(),
                    amount: Uint128::from(1000u128),
                }],
            );
            let seller_info = message_info(&Addr::unchecked("seller"), &[]);

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token_denom".to_string(),
            };

            let token_pair = TokenPair {
                base_token: "base_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
                enabled: true,
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();
            TOKEN_PAIRS
                .save(deps.as_mut().storage, "pair_id".to_string(), &token_pair)
                .unwrap();

            ORDER_BOOKS
                .save(
                    deps.as_mut().storage,
                    "pair_id".to_string(),
                    &OrderBook {
                        pair_id: "pair_id".to_string(),
                        buy_orders: BTreeMap::new(),
                        sell_orders: BTreeMap::new(),
                    },
                )
                .unwrap();

            NEXT_ORDER_ID.save(deps.as_mut().storage, &0u64).unwrap();
            NEXT_TRADE_ID.save(deps.as_mut().storage, &0u64).unwrap();

            let amount = Uint128::from(100u128);
            let price = Uint128::from(10u128);

            // Mock balance and allowance queries for CW20 tokens
            let sell_info = message_info(&Addr::unchecked("seller"), &[]);
            let cw20_token_address = String::from("quote_token");

            // Clone necessary variables to avoid moving them into the closure
            let cw20_token_address_clone = cw20_token_address.clone();
            let sell_info_sender_clone = sell_info.sender.clone();

            // Mock the balance and allowance queries
            deps.querier.update_wasm(move |query| {
                let cw20_token_address = cw20_token_address_clone.clone();
                let sell_info_sender = sell_info_sender_clone.clone();
                let env = mock_env();

                match query {
                    WasmQuery::Smart { contract_addr, msg } => {
                        if contract_addr == &cw20_token_address.clone() {
                            if let Ok(cw20::Cw20QueryMsg::Balance { address }) = from_json(&msg) {
                                if address == sell_info_sender.into_string() {
                                    return SystemResult::Ok(ContractResult::Ok(
                                        to_json_binary(&BalanceResponse {
                                            balance: Uint128::from(1000u128),
                                        })
                                        .unwrap(),
                                    ));
                                }
                            } else if let Ok(cw20::Cw20QueryMsg::Allowance { owner, spender }) =
                                from_json(&msg)
                            {
                                if owner == sell_info_sender.into_string()
                                    && spender == env.contract.address.to_string()
                                {
                                    return SystemResult::Ok(ContractResult::Ok(
                                        to_json_binary(&AllowanceResponse {
                                            allowance: Uint128::from(1000u128),
                                            expires: Expiration::Never {},
                                        })
                                        .unwrap(),
                                    ));
                                }
                            }
                        }
                        SystemResult::Err(SystemError::UnsupportedRequest {
                            kind: "".to_string(),
                        })
                    }
                    _ => SystemResult::Err(SystemError::UnsupportedRequest {
                        kind: "".to_string(),
                    }),
                }
            });

            // Place buy order
            let buy_res = execute_place_limit_order(
                deps.as_mut(),
                env.clone(),
                buyer_info.clone(),
                "pair_id".to_string(),
                amount,
                price,
                true,
            )
            .unwrap();

            // Place sell order
            let sell_res = execute_place_limit_order(
                deps.as_mut(),
                env.clone(),
                seller_info.clone(),
                "pair_id".to_string(),
                amount,
                price,
                false,
            )
            .unwrap();

            // Verify response attributes for buy order
            assert_eq!(buy_res.attributes.len(), 6);
            assert_eq!(buy_res.attributes[0].key, "action");
            assert_eq!(buy_res.attributes[0].value, "place_limit_order");
            assert_eq!(buy_res.attributes[1].key, "order_id");
            assert_eq!(buy_res.attributes[1].value, "0");
            assert_eq!(buy_res.attributes[2].key, "pair_id");
            assert_eq!(buy_res.attributes[2].value, "pair_id");
            assert_eq!(buy_res.attributes[3].key, "is_buy");
            assert_eq!(buy_res.attributes[3].value, "true");
            assert_eq!(buy_res.attributes[4].key, "amount");
            assert_eq!(buy_res.attributes[4].value, amount.to_string());
            assert_eq!(buy_res.attributes[5].key, "price");
            assert_eq!(buy_res.attributes[5].value, price.to_string());

            // Verify response attributes for sell order
            assert_eq!(sell_res.attributes.len(), 6);
            assert_eq!(sell_res.attributes[0].key, "action");
            assert_eq!(sell_res.attributes[0].value, "place_limit_order");
            assert_eq!(sell_res.attributes[1].key, "order_id");
            assert_eq!(sell_res.attributes[1].value, "1");
            assert_eq!(sell_res.attributes[2].key, "pair_id");
            assert_eq!(sell_res.attributes[2].value, "pair_id");
            assert_eq!(sell_res.attributes[3].key, "is_buy");
            assert_eq!(sell_res.attributes[3].value, "false");
            assert_eq!(sell_res.attributes[4].key, "amount");
            assert_eq!(sell_res.attributes[4].value, amount.to_string());
            assert_eq!(sell_res.attributes[5].key, "price");
            assert_eq!(sell_res.attributes[5].value, price.to_string());

            // Verify that orders have been matched and updated in storage
            let saved_buy_order = USER_ORDERS
                .load(&deps.storage, (Addr::unchecked("buyer"), 0))
                .unwrap();
            assert_eq!(saved_buy_order.filled_amount, amount);
            assert_eq!(saved_buy_order.remaining_amount, Uint128::zero());
            assert_eq!(saved_buy_order.status, OrderStatus::Filled);

            let saved_sell_order = USER_ORDERS
                .load(&deps.storage, (Addr::unchecked("seller"), 1))
                .unwrap();
            assert_eq!(saved_sell_order.filled_amount, amount);
            assert_eq!(saved_sell_order.remaining_amount, Uint128::zero());
            assert_eq!(saved_sell_order.status, OrderStatus::Filled);

            // Verify the order book is empty after matching orders
            let order_book = ORDER_BOOKS
                .load(&deps.storage, "pair_id".to_string())
                .unwrap();
            assert!(order_book.buy_orders.is_empty());
            assert!(order_book.sell_orders.is_empty());
        }

        #[test]
        fn test_execute_place_limit_order_trading_disabled() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let info = message_info(
                &Addr::unchecked("buyer"),
                &[Coin {
                    denom: "base_token".to_string(),
                    amount: Uint128::from(1000u128),
                }],
            );

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: false, // Trading disabled
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token".to_string(),
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();

            let amount = Uint128::from(100u128);
            let price = Uint128::from(10u128);
            let is_buy = true;

            // Execute the place limit order function
            let res = execute_place_limit_order(
                deps.as_mut(),
                env,
                info.clone(),
                "pair_id".to_string(),
                amount,
                price,
                is_buy,
            );

            // Verify that the function returned an error
            assert!(res.is_err());
            assert_eq!(
                res.unwrap_err(),
                StdError::generic_err("Trading is currently disabled")
            );
        }

        #[test]
        fn test_execute_place_limit_order_trading_pair_disabled() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let info = message_info(
                &Addr::unchecked("buyer"),
                &[Coin {
                    denom: "base_token".to_string(),
                    amount: Uint128::from(1000u128),
                }],
            );

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token".to_string(),
            };

            let token_pair = TokenPair {
                base_token: "base_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
                enabled: false, // Trading pair disabled
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();
            TOKEN_PAIRS
                .save(deps.as_mut().storage, "pair_id".to_string(), &token_pair)
                .unwrap();

            let amount = Uint128::from(100u128);
            let price = Uint128::from(10u128);
            let is_buy = true;

            // Execute the place limit order function
            let res = execute_place_limit_order(
                deps.as_mut(),
                env,
                info.clone(),
                "pair_id".to_string(),
                amount,
                price,
                is_buy,
            );

            // Verify that the function returned an error
            assert!(res.is_err());
            assert_eq!(
                res.unwrap_err(),
                StdError::generic_err("Trading pair is disabled")
            );
        }

        #[test]
        fn test_match_orders_with_matching_orders() {
            let mut deps = mock_dependencies();
            let env = mock_env();

            // Initialize the token pair and config
            let token_pair = TokenPair {
                base_token: "base_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
                enabled: true,
            };

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token_denom".to_string(),
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();
            TOKEN_PAIRS
                .save(deps.as_mut().storage, "pair_id".to_string(), &token_pair)
                .unwrap();

            // Create a buy order and a sell order
            let buy_order = Order {
                id: 1,
                owner: Addr::unchecked("buyer"),
                pair_id: "pair_id".to_string(),
                token_amount: Uint128::from(100u128),
                price: Uint128::from(10u128),
                timestamp: env.block.time.seconds() as u64,
                status: OrderStatus::Active,
                filled_amount: Uint128::zero(),
                remaining_amount: Uint128::from(100u128),
                order_type: OrderType::Buy,
                created_at: env.block.height,
            };

            let sell_order = Order {
                id: 2,
                owner: Addr::unchecked("seller"),
                pair_id: "pair_id".to_string(),
                token_amount: Uint128::from(100u128),
                price: Uint128::from(10u128),
                timestamp: env.block.time.seconds() as u64,
                status: OrderStatus::Active,
                filled_amount: Uint128::zero(),
                remaining_amount: Uint128::from(100u128),
                order_type: OrderType::Sell,
                created_at: env.block.height,
            };

            // Initialize the order book
            let mut order_book = OrderBook {
                pair_id: "pair_id".to_string(),
                buy_orders: BTreeMap::new(),
                sell_orders: BTreeMap::new(),
            };

            order_book
                .buy_orders
                .entry(10)
                .or_insert_with(Vec::new)
                .push(buy_order.clone());
            order_book
                .sell_orders
                .entry(10)
                .or_insert_with(Vec::new)
                .push(sell_order.clone());

            ORDER_BOOKS
                .save(deps.as_mut().storage, "pair_id".to_string(), &order_book)
                .unwrap();

            NEXT_TRADE_ID.save(deps.as_mut().storage, &0u64).unwrap();

            // Execute the match_orders function
            let res = match_orders(deps.as_mut(), &env, "pair_id".to_string(), true).unwrap();

            // Verify response attributes
            assert_eq!(res.attributes.len(), 13);
            assert_eq!(res.attributes[0].key, "event_type");
            assert_eq!(res.attributes[0].value, "trade");

            // Verify that orders have been matched and updated in storage
            let saved_buy_order = USER_ORDERS
                .load(&deps.storage, (Addr::unchecked("buyer"), 1))
                .unwrap();
            assert_eq!(saved_buy_order.filled_amount, Uint128::from(100u128));
            assert_eq!(saved_buy_order.remaining_amount, Uint128::zero());
            assert_eq!(saved_buy_order.status, OrderStatus::Filled);

            let saved_sell_order = USER_ORDERS
                .load(&deps.storage, (Addr::unchecked("seller"), 2))
                .unwrap();
            assert_eq!(saved_sell_order.filled_amount, Uint128::from(100u128));
            assert_eq!(saved_sell_order.remaining_amount, Uint128::zero());
            assert_eq!(saved_sell_order.status, OrderStatus::Filled);

            // Order book should be emptied
            let order_book = ORDER_BOOKS
                .load(&mut deps.storage, "pair_id".to_string())
                .unwrap();
            assert!(order_book.buy_orders.is_empty());
            assert!(order_book.sell_orders.is_empty());
        }

        #[test]
        fn test_match_orders_with_no_matching_orders() {
            let mut deps = mock_dependencies();
            let env = mock_env();

            // Initialize the token pair and config
            let token_pair = TokenPair {
                base_token: "base_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
                enabled: true,
            };

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token_denom".to_string(),
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();
            TOKEN_PAIRS
                .save(deps.as_mut().storage, "pair_id".to_string(), &token_pair)
                .unwrap();

            // Create a buy order and a sell order with non-matching prices
            let buy_order = Order {
                id: 1,
                owner: Addr::unchecked("buyer"),
                pair_id: "pair_id".to_string(),
                token_amount: Uint128::from(100u128),
                price: Uint128::from(10u128),
                timestamp: env.block.time.seconds() as u64,
                status: OrderStatus::Active,
                filled_amount: Uint128::zero(),
                remaining_amount: Uint128::from(100u128),
                order_type: OrderType::Buy,
                created_at: env.block.height,
            };

            let sell_order = Order {
                id: 2,
                owner: Addr::unchecked("seller"),
                pair_id: "pair_id".to_string(),
                token_amount: Uint128::from(100u128),
                price: Uint128::from(15u128),
                timestamp: env.block.time.seconds() as u64,
                status: OrderStatus::Active,
                filled_amount: Uint128::zero(),
                remaining_amount: Uint128::from(100u128),
                order_type: OrderType::Sell,
                created_at: env.block.height,
            };

            // Save orders to USER_ORDERS storage
            USER_ORDERS
                .save(
                    deps.as_mut().storage,
                    (buy_order.owner.clone(), buy_order.id),
                    &buy_order,
                )
                .unwrap();
            USER_ORDERS
                .save(
                    deps.as_mut().storage,
                    (sell_order.owner.clone(), sell_order.id),
                    &sell_order,
                )
                .unwrap();

            // Initialize the order book
            let mut order_book = OrderBook {
                pair_id: "pair_id".to_string(),
                buy_orders: BTreeMap::new(),
                sell_orders: BTreeMap::new(),
            };

            order_book
                .buy_orders
                .entry(10)
                .or_insert_with(Vec::new)
                .push(buy_order.clone());
            order_book
                .sell_orders
                .entry(15)
                .or_insert_with(Vec::new)
                .push(sell_order.clone());

            ORDER_BOOKS
                .save(deps.as_mut().storage, "pair_id".to_string(), &order_book)
                .unwrap();

            NEXT_TRADE_ID.save(deps.as_mut().storage, &0u64).unwrap();

            // Execute the match_orders function
            let res = match_orders(deps.as_mut(), &env, "pair_id".to_string(), true).unwrap();

            // Verify response attributes
            assert_eq!(res.attributes.is_empty(), true);

            // Verify that orders have not been matched and remain unchanged
            let saved_buy_order = USER_ORDERS
                .load(&deps.storage, (Addr::unchecked("buyer"), 1))
                .unwrap();
            assert_eq!(saved_buy_order.filled_amount, Uint128::zero());
            assert_eq!(saved_buy_order.remaining_amount, Uint128::from(100u128));
            assert_eq!(saved_buy_order.status, OrderStatus::Active);

            let saved_sell_order = USER_ORDERS
                .load(&deps.storage, (Addr::unchecked("seller"), 2))
                .unwrap();
            assert_eq!(saved_sell_order.filled_amount, Uint128::zero());
            assert_eq!(saved_sell_order.remaining_amount, Uint128::from(100u128));
            assert_eq!(saved_sell_order.status, OrderStatus::Active);
        }

        #[test]
        fn test_execute_place_limit_order_buy_partial_fill() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let info = message_info(
                &Addr::unchecked("buyer"),
                &[Coin {
                    denom: "ubase_token".to_string(),
                    amount: Uint128::new(500),
                }],
            );

            // Initialize the token pair and config
            let token_pair = TokenPair {
                base_token: "ubase_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
                enabled: true,
            };

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token".to_string(),
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();
            TOKEN_PAIRS
                .save(deps.as_mut().storage, "pair_id".to_string(), &token_pair)
                .unwrap();

            // Initially empty order book and order ID
            let order_book = OrderBook {
                pair_id: "pair_id".to_string(),
                buy_orders: BTreeMap::new(),
                sell_orders: BTreeMap::new(),
            };
            ORDER_BOOKS
                .save(deps.as_mut().storage, "pair_id".to_string(), &order_book)
                .unwrap();
            NEXT_ORDER_ID.save(deps.as_mut().storage, &1).unwrap();
            NEXT_TRADE_ID.save(deps.as_mut().storage, &1).unwrap();

            // Mock balance and allowance queries for CW20 tokens
            let sell_info = message_info(&Addr::unchecked("seller"), &[]);
            let cw20_token_address = String::from("quote_token");

            // Clone necessary variables to avoid moving them into the closure
            let cw20_token_address_clone = cw20_token_address.clone();
            let sell_info_sender_clone = sell_info.sender.clone();

            // Mock the balance and allowance queries
            deps.querier.update_wasm(move |query| {
                let cw20_token_address = cw20_token_address_clone.clone();
                let sell_info_sender = sell_info_sender_clone.clone();
                let env = mock_env();

                match query {
                    WasmQuery::Smart { contract_addr, msg } => {
                        if contract_addr == &cw20_token_address.clone() {
                            if let Ok(cw20::Cw20QueryMsg::Balance { address }) = from_json(&msg) {
                                if address == sell_info_sender.into_string() {
                                    return SystemResult::Ok(ContractResult::Ok(
                                        to_json_binary(&cw20::BalanceResponse {
                                            balance: Uint128::from(1000u128),
                                        })
                                        .unwrap(),
                                    ));
                                }
                            } else if let Ok(cw20::Cw20QueryMsg::Allowance { owner, spender }) =
                                from_json(&msg)
                            {
                                if owner == sell_info_sender.into_string()
                                    && spender == env.contract.address.to_string()
                                {
                                    return SystemResult::Ok(ContractResult::Ok(
                                        to_json_binary(&cw20::AllowanceResponse {
                                            allowance: Uint128::from(1000u128),
                                            expires: cw20::Expiration::Never {},
                                        })
                                        .unwrap(),
                                    ));
                                }
                            }
                        }
                        SystemResult::Err(SystemError::UnsupportedRequest {
                            kind: "".to_string(),
                        })
                    }
                    _ => SystemResult::Err(SystemError::UnsupportedRequest {
                        kind: "".to_string(),
                    }),
                }
            });

            // Place the first limit sell order
            let sell_info = message_info(&Addr::unchecked("seller"), &[]);
            execute_place_limit_order(
                deps.as_mut(),
                env.clone(),
                sell_info,
                "pair_id".to_string(),
                Uint128::new(1000), // Sell order amount
                Uint128::new(1),    // Price
                false,
            )
            .unwrap();

            // Place a buy order that partially fills the sell order
            let response = execute_place_limit_order(
                deps.as_mut(),
                env.clone(),
                info,
                "pair_id".to_string(),
                Uint128::new(500), // Buy order amount
                Uint128::new(1),   // Price
                true,
            )
            .unwrap();

            // Verify response attributes
            assert_eq!(response.attributes.len(), 6);
            assert_eq!(response.attributes[0], attr("action", "place_limit_order"));
            assert_eq!(response.attributes[1], attr("order_id", "2"));
            assert_eq!(response.attributes[2], attr("pair_id", "pair_id"));
            assert_eq!(response.attributes[3], attr("is_buy", "true"));
            assert_eq!(response.attributes[4], attr("amount", "500"));
            assert_eq!(response.attributes[5], attr("price", "1"));

            // Verify that the sell order is partially filled
            let sell_order: Order = USER_ORDERS
                .load(deps.as_mut().storage, (Addr::unchecked("seller"), 1))
                .unwrap();
            assert_eq!(sell_order.filled_amount, Uint128::new(500));
            assert_eq!(sell_order.remaining_amount, Uint128::new(500));
            assert_eq!(sell_order.status, OrderStatus::Active); // Still active because it's partially filled

            // Verify that the buy order is fully filled
            let buy_order: Order = USER_ORDERS
                .load(deps.as_mut().storage, (Addr::unchecked("buyer"), 2))
                .unwrap();
            assert_eq!(buy_order.filled_amount, Uint128::new(500));
            assert_eq!(buy_order.remaining_amount, Uint128::zero());
            assert_eq!(buy_order.status, OrderStatus::Filled);
        }

        #[test]
        fn test_execute_place_limit_order_sell_partial_fill() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let info = message_info(&Addr::unchecked("seller"), &[]);

            // Initialize the token pair and config
            let token_pair = TokenPair {
                base_token: "ubase_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
                enabled: true,
            };

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token".to_string(),
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();
            TOKEN_PAIRS
                .save(deps.as_mut().storage, "pair_id".to_string(), &token_pair)
                .unwrap();

            // Initially empty order book and order ID
            let order_book = OrderBook {
                pair_id: "pair_id".to_string(),
                buy_orders: BTreeMap::new(),
                sell_orders: BTreeMap::new(),
            };
            ORDER_BOOKS
                .save(deps.as_mut().storage, "pair_id".to_string(), &order_book)
                .unwrap();

            // Initialize NEXT_ORDER_ID and NEXT_TRADE_ID
            NEXT_ORDER_ID.save(deps.as_mut().storage, &1).unwrap();
            NEXT_TRADE_ID.save(deps.as_mut().storage, &1).unwrap();

            // Mock balance and allowance queries for CW20 tokens
            let buy_info = message_info(
                &Addr::unchecked("buyer"),
                &[Coin {
                    denom: "ubase_token".to_string(),
                    amount: Uint128::new(500),
                }],
            );

            // Mock balance and allowance queries for CW20 tokens
            let sell_info = message_info(&Addr::unchecked("seller"), &[]);
            let cw20_token_address = String::from("quote_token");

            // Clone necessary variables to avoid moving them into the closure
            let cw20_token_address_clone = cw20_token_address.clone();
            let sell_info_sender_clone = sell_info.sender.clone();

            // Mock the balance and allowance queries
            deps.querier.update_wasm(move |query| {
                let cw20_token_address = cw20_token_address_clone.clone();
                let sell_info_sender = sell_info_sender_clone.clone();
                let env = mock_env();

                match query {
                    WasmQuery::Smart { contract_addr, msg } => {
                        if contract_addr == &cw20_token_address.clone() {
                            if let Ok(cw20::Cw20QueryMsg::Balance { address }) = from_json(&msg) {
                                if address == sell_info_sender.into_string() {
                                    return SystemResult::Ok(ContractResult::Ok(
                                        to_json_binary(&cw20::BalanceResponse {
                                            balance: Uint128::from(1000u128),
                                        })
                                        .unwrap(),
                                    ));
                                }
                            } else if let Ok(cw20::Cw20QueryMsg::Allowance { owner, spender }) =
                                from_json(&msg)
                            {
                                if owner == sell_info_sender.into_string()
                                    && spender == env.contract.address.to_string()
                                {
                                    return SystemResult::Ok(ContractResult::Ok(
                                        to_json_binary(&cw20::AllowanceResponse {
                                            allowance: Uint128::from(1000u128),
                                            expires: cw20::Expiration::Never {},
                                        })
                                        .unwrap(),
                                    ));
                                }
                            }
                        }
                        SystemResult::Err(SystemError::UnsupportedRequest {
                            kind: "".to_string(),
                        })
                    }
                    _ => SystemResult::Err(SystemError::UnsupportedRequest {
                        kind: "".to_string(),
                    }),
                }
            });

            // Place the first limit buy order
            execute_place_limit_order(
                deps.as_mut(),
                env.clone(),
                buy_info,
                "pair_id".to_string(),
                Uint128::new(500), // Buy order amount
                Uint128::new(1),   // Price
                true,
            )
            .unwrap();

            // Place a sell order that partially fills the buy order
            let response = execute_place_limit_order(
                deps.as_mut(),
                env.clone(),
                info,
                "pair_id".to_string(),
                Uint128::new(1000), // Sell order amount
                Uint128::new(1),    // Price
                false,
            )
            .unwrap();

            // Verify response attributes
            assert_eq!(response.attributes.len(), 6);
            assert_eq!(response.attributes[0], attr("action", "place_limit_order"));
            assert_eq!(response.attributes[1], attr("order_id", "2"));
            assert_eq!(response.attributes[2], attr("pair_id", "pair_id"));
            assert_eq!(response.attributes[3], attr("is_buy", "false"));
            assert_eq!(response.attributes[4], attr("amount", "1000"));
            assert_eq!(response.attributes[5], attr("price", "1"));

            // Verify that the buy order is fully filled
            let buy_order: Order = USER_ORDERS
                .load(deps.as_mut().storage, (Addr::unchecked("buyer"), 1))
                .unwrap();
            assert_eq!(buy_order.filled_amount, Uint128::new(500));
            assert_eq!(buy_order.remaining_amount, Uint128::zero());
            assert_eq!(buy_order.status, OrderStatus::Filled);

            // Verify that the sell order is partially filled
            let sell_order: Order = USER_ORDERS
                .load(deps.as_mut().storage, (Addr::unchecked("seller"), 2))
                .unwrap();
            assert_eq!(sell_order.filled_amount, Uint128::new(500));
            assert_eq!(sell_order.remaining_amount, Uint128::new(500));
            assert_eq!(sell_order.status, OrderStatus::Active); // Still active because it's partially filled
        }

        #[test]
        fn test_validate_and_handle_native_tokens() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let info = message_info(
                &Addr::unchecked("buyer"),
                &[Coin {
                    denom: "base_token".to_string(),
                    amount: Uint128::from(1000u128),
                }],
            );

            let token_pair = TokenPair {
                base_token: "base_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
                enabled: true,
            };

            let result = validate_and_handle_tokens(
                &deps.as_mut(),
                &env,
                &info,
                &token_pair,
                Uint128::from(100u128),
                Uint128::from(10u128),
                true,
            );
            assert!(result.is_ok());
        }

        #[test]
        fn test_validate_and_handle_cw20_tokens() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let info = message_info(&Addr::unchecked("seller"), &[]);

            let token_pair = TokenPair {
                base_token: "base_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
                enabled: true,
            };

            // Mock balance and allowance queries for CW20 tokens
            let sell_info = message_info(&Addr::unchecked("seller"), &[]);
            let cw20_token_address = String::from("quote_token");

            // Clone necessary variables to avoid moving them into the closure
            let cw20_token_address_clone = cw20_token_address.clone();
            let sell_info_sender_clone = sell_info.sender.clone();

            // Mock the balance and allowance queries
            deps.querier.update_wasm(move |query| {
                let cw20_token_address = cw20_token_address_clone.clone();
                let sell_info_sender = sell_info_sender_clone.clone();
                let env = mock_env();

                match query {
                    WasmQuery::Smart { contract_addr, msg } => {
                        if contract_addr == &cw20_token_address.clone() {
                            if let Ok(cw20::Cw20QueryMsg::Balance { address }) = from_json(&msg) {
                                if address == sell_info_sender.into_string() {
                                    return SystemResult::Ok(ContractResult::Ok(
                                        to_json_binary(&BalanceResponse {
                                            balance: Uint128::from(1000u128),
                                        })
                                        .unwrap(),
                                    ));
                                }
                            } else if let Ok(cw20::Cw20QueryMsg::Allowance { owner, spender }) =
                                from_json(&msg)
                            {
                                if owner == sell_info_sender.into_string()
                                    && spender == env.contract.address.to_string()
                                {
                                    return SystemResult::Ok(ContractResult::Ok(
                                        to_json_binary(&AllowanceResponse {
                                            allowance: Uint128::from(1000u128),
                                            expires: Expiration::Never {},
                                        })
                                        .unwrap(),
                                    ));
                                }
                            }
                        }
                        SystemResult::Err(SystemError::UnsupportedRequest {
                            kind: "".to_string(),
                        })
                    }
                    _ => SystemResult::Err(SystemError::UnsupportedRequest {
                        kind: "".to_string(),
                    }),
                }
            });

            // Test case for selling CW20 tokens
            let result = validate_and_handle_tokens(
                &deps.as_mut(),
                &env,
                &info,
                &token_pair,
                Uint128::from(100u128),
                Uint128::from(10u128),
                false,
            );
            assert!(result.is_ok());
        }

        #[test]
        fn test_validate_native_token_payment_insufficient_funds() {
            let info = message_info(
                &Addr::unchecked("buyer"),
                &[Coin {
                    denom: "quote_token".to_string(),
                    amount: Uint128::from(50u128),
                }],
            );

            let result =
                validate_native_token_payment(&info, "quote_token", Uint128::from(100u128));
            assert!(result.is_err());
        }

        #[test]
        fn test_validate_native_token_payment_excess_funds() {
            let info = message_info(
                &Addr::unchecked("buyer"),
                &[Coin {
                    denom: "quote_token".to_string(),
                    amount: Uint128::from(150u128),
                }],
            );

            let result =
                validate_native_token_payment(&info, "quote_token", Uint128::from(100u128));
            assert!(result.is_err());
        }

        #[test]
        fn test_validate_cw20_token_payment_insufficient_allowance() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let info = message_info(&Addr::unchecked("seller"), &[]);

            // Mock balance and allowance queries for CW20 tokens
            let sell_info = message_info(&Addr::unchecked("seller"), &[]);
            let cw20_token_address = String::from("base_token");

            // Clone necessary variables to avoid moving them into the closure
            let cw20_token_address_clone = cw20_token_address.clone();
            let sell_info_sender_clone = sell_info.sender.clone();

            // Mock the balance and allowance queries
            deps.querier.update_wasm(move |query| {
                let cw20_token_address = cw20_token_address_clone.clone();
                let sell_info_sender = sell_info_sender_clone.clone();
                let env = mock_env();

                match query {
                    WasmQuery::Smart { contract_addr, msg } => {
                        if contract_addr == &cw20_token_address.clone() {
                            if let Ok(cw20::Cw20QueryMsg::Allowance { owner, spender }) =
                                from_json(&msg)
                            {
                                if owner == sell_info_sender.into_string()
                                    && spender == env.contract.address.to_string()
                                {
                                    return SystemResult::Ok(ContractResult::Ok(
                                        to_json_binary(&AllowanceResponse {
                                            allowance: Uint128::from(50u128),
                                            expires: Expiration::Never {},
                                        })
                                        .unwrap(),
                                    ));
                                }
                            }
                        }
                        SystemResult::Err(SystemError::UnsupportedRequest {
                            kind: "".to_string(),
                        })
                    }
                    _ => SystemResult::Err(SystemError::UnsupportedRequest {
                        kind: "".to_string(),
                    }),
                }
            });

            let result = validate_cw20_token_payment(
                &deps.as_ref(),
                &env,
                &info,
                "base_token",
                Uint128::from(100u128),
            );
            assert!(result.is_err());
        }

        #[test]
        fn test_validate_cw20_token_payment_insufficient_balance() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let info = message_info(&Addr::unchecked("seller"), &[]);

            // Mock balance and allowance queries for CW20 tokens
            let sell_info = message_info(&Addr::unchecked("seller"), &[]);
            let cw20_token_address = String::from("base_token");

            // Clone necessary variables to avoid moving them into the closure
            let cw20_token_address_clone = cw20_token_address.clone();
            let sell_info_sender_clone = sell_info.sender.clone();

            // Mock the balance and allowance queries
            deps.querier.update_wasm(move |query| {
                let cw20_token_address = cw20_token_address_clone.clone();
                let sell_info_sender = sell_info_sender_clone.clone();

                match query {
                    WasmQuery::Smart { contract_addr, msg } => {
                        if contract_addr == &cw20_token_address.clone() {
                            if let Ok(cw20::Cw20QueryMsg::Balance { address }) = from_json(&msg) {
                                if address == sell_info_sender.into_string() {
                                    return SystemResult::Ok(ContractResult::Ok(
                                        to_json_binary(&BalanceResponse {
                                            balance: Uint128::from(50u128),
                                        })
                                        .unwrap(),
                                    ));
                                }
                            }
                        }
                        SystemResult::Err(SystemError::UnsupportedRequest {
                            kind: "".to_string(),
                        })
                    }
                    _ => SystemResult::Err(SystemError::UnsupportedRequest {
                        kind: "".to_string(),
                    }),
                }
            });

            let result = validate_cw20_token_payment(
                &deps.as_ref(),
                &env,
                &info,
                "base_token",
                Uint128::from(100u128),
            );
            assert!(result.is_err());
        }

        #[test]
        fn test_execute_cancel_order() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let info = message_info(&Addr::unchecked("buyer"), &[]);

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token_denom".to_string(),
            };

            let token_pair = TokenPair {
                base_token: "base_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
                enabled: true,
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();
            TOKEN_PAIRS
                .save(deps.as_mut().storage, "pair_id".to_string(), &token_pair)
                .unwrap();

            let mut order_book = OrderBook {
                pair_id: "pair_id".to_string(),
                buy_orders: BTreeMap::new(),
                sell_orders: BTreeMap::new(),
            };

            let order = Order {
                id: 1,
                owner: Addr::unchecked("buyer"),
                pair_id: "pair_id".to_string(),
                token_amount: Uint128::from(100u128),
                price: Uint128::from(10u128),
                timestamp: env.block.time.seconds() as u64,
                status: OrderStatus::Active,
                filled_amount: Uint128::zero(),
                remaining_amount: Uint128::from(100u128),
                order_type: OrderType::Buy,
                created_at: env.block.height,
            };

            // Add the order to the order book and save the state
            order_book
                .buy_orders
                .entry(order.price.u128())
                .or_insert_with(Vec::new)
                .push(order.clone());
            ORDER_BOOKS
                .save(deps.as_mut().storage, "pair_id".to_string(), &order_book)
                .unwrap();
            USER_ORDERS
                .save(
                    deps.as_mut().storage,
                    (info.sender.clone(), order.id),
                    &order,
                )
                .unwrap();

            // Execute the cancel order function
            let res = execute_cancel_order(
                deps.as_mut(),
                env,
                info.clone(),
                order.id,
                "pair_id".to_string(),
            )
            .unwrap();

            // Verify response attributes
            assert_eq!(res.attributes.len(), 2);
            assert_eq!(res.attributes[0].key, "action");
            assert_eq!(res.attributes[0].value, "cancel_order");
            assert_eq!(res.attributes[1].key, "order_id");
            assert_eq!(res.attributes[1].value, order.id.to_string());

            // Verify the order status is updated to "Cancelled"
            let updated_order = USER_ORDERS
                .load(&deps.storage, (info.sender.clone(), order.id))
                .unwrap();
            assert_eq!(updated_order.status, OrderStatus::Cancelled);

            // Verify the order is removed from the order book
            let updated_order_book = ORDER_BOOKS
                .load(&deps.storage, "pair_id".to_string())
                .unwrap();
            assert!(updated_order_book
                .buy_orders
                .get(&order.price.u128())
                .unwrap()
                .is_empty());
        }

        #[test]
        fn test_execute_cancel_order_not_owner() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let info = message_info(&Addr::unchecked("buyer"), &[]);
            let wrong_info = message_info(&Addr::unchecked("wrong_buyer"), &[]);

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token".to_string(),
            };

            let token_pair = TokenPair {
                base_token: "base_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
                enabled: true,
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();
            TOKEN_PAIRS
                .save(deps.as_mut().storage, "pair_id".to_string(), &token_pair)
                .unwrap();

            let mut order_book = OrderBook {
                pair_id: "pair_id".to_string(),
                buy_orders: BTreeMap::new(),
                sell_orders: BTreeMap::new(),
            };

            let order = Order {
                id: 1,
                owner: Addr::unchecked("buyer"),
                pair_id: "pair_id".to_string(),
                token_amount: Uint128::from(100u128),
                price: Uint128::from(10u128),
                timestamp: env.block.time.seconds() as u64,
                status: OrderStatus::Active,
                filled_amount: Uint128::zero(),
                remaining_amount: Uint128::from(100u128),
                order_type: OrderType::Buy,
                created_at: env.block.height,
            };

            // Add the order to the order book and save the state
            order_book
                .buy_orders
                .entry(order.price.u128())
                .or_insert_with(Vec::new)
                .push(order.clone());
            ORDER_BOOKS
                .save(deps.as_mut().storage, "pair_id".to_string(), &order_book)
                .unwrap();
            USER_ORDERS
                .save(
                    deps.as_mut().storage,
                    (info.sender.clone(), order.id),
                    &order,
                )
                .unwrap();

            // Attempt to cancel the order with the wrong owner
            let res = execute_cancel_order(
                deps.as_mut(),
                env,
                wrong_info.clone(),
                order.id,
                "pair_id".to_string(),
            );

            // Verify that the function returned an error
            assert!(res.is_err());
            assert_eq!(res.unwrap_err(), StdError::not_found("Order"));
        }

        #[test]
        fn test_execute_native_transfer() {
            let denom = "uatom";
            let to = Addr::unchecked("recipient");
            let amount = Uint128::from(100u128);

            let msg = execute_native_transfer(denom, &to, amount).unwrap();

            // Expected BankMsg
            let expected_msg = CosmosMsg::Bank(BankMsg::Send {
                to_address: to.to_string(),
                amount: vec![Coin {
                    denom: denom.to_string(),
                    amount,
                }],
            });

            assert_eq!(msg, expected_msg);
        }

        #[test]
        fn test_execute_cw20_transfer() {
            let token_address = "token_contract";
            let from = Addr::unchecked("sender");
            let to = Addr::unchecked("recipient");
            let amount = Uint128::from(100u128);

            let msg = execute_cw20_transfer(token_address, &from, &to, amount).unwrap();

            // Expected WasmMsg
            let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_address.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: from.to_string(),
                    recipient: to.to_string(),
                    amount,
                })
                .unwrap(),
                funds: vec![],
            });

            assert_eq!(msg, expected_msg);
        }

        #[test]
        fn test_calculate_exp() {
            let x = Decimal::from_str("1").unwrap();

            // Execute the function
            let alpha = Decimal::from_str("0.1").unwrap();

            // Execute the function and expect an error
            let res = calculate_ema_exp(x, alpha);

            // Verify the exponent is calculated correctly
            assert!(res.is_ok());
        }

        #[test]
        fn test_calculate_exponential_price_buy() {
            let mut deps = mock_dependencies();

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token".to_string(),
            };

            let token_pair = TokenPair {
                enabled: true,
                base_token: "ubase_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
            };

            let pool = Pool {
                enabled: true,
                token_sold: Uint128::new(100_000),
                pair_id: "pair_id".to_string(),
                curve_slope: Uint128::from(1u128),
                token_address: Addr::unchecked("token_address"),
                total_reserve_token: Uint128::from(1_000u128),
                total_volume: Uint128::from(100_000u128),
                total_trades: Uint128::from(100u128),
                total_fees_collected: Uint128::from(10u128),
                last_price: Uint128::from(5u128),
            };

            let token_info = TokenInfo {
                name: "Test Token".to_owned(),
                symbol: "TST".to_owned(),
                decimals: 9,
                total_supply: 100_000_000_000u128.into(),
                initial_price: BASE_PRICE.into(),
                max_price_impact: Uint128::from(30u128),
                graduated: false,
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();
            TOKEN_PAIRS
                .save(deps.as_mut().storage, "pair_id".to_string(), &token_pair)
                .unwrap();
            POOLS
                .save(deps.as_mut().storage, "token_address".to_string(), &pool)
                .unwrap();
            TOKEN_INFO
                .save(
                    deps.as_mut().storage,
                    "token_address".to_string(),
                    &token_info,
                )
                .unwrap();

            let base_price = Decimal::from_ratio(BASE_PRICE, Uint128::new(1_000_000));
            let slope = Decimal::from_ratio(pool.curve_slope, Uint128::new(1_000_000));

            let current_supply = Uint128::new(100_000);
            let amount = Uint128::new(1_000);
            let lower_bound = current_supply;
            let upper_bound = current_supply + amount;

            let lower_dec = Decimal::from_ratio(lower_bound, Uint128::new(1));
            let upper_dec = Decimal::from_ratio(upper_bound, Uint128::new(1));
            let alpha = Decimal::from_str("0.1").unwrap();

            let exp_upper = calculate_ema_exp(slope * upper_dec, alpha).unwrap();
            let exp_lower = calculate_ema_exp(slope * lower_dec, alpha).unwrap();

            let avg_price = base_price * (exp_upper - exp_lower)
                / (slope * Decimal::from_ratio(amount, Uint128::new(1)));

            let expected_price = Uint128::new(
                (avg_price * Decimal::from_ratio(10_u128.pow(token_info.decimals as u32), 1u128))
                    .to_uint_floor()
                    .into(),
            );

            let price = calculate_exponential_price(
                deps.as_mut().storage,
                "token_address".to_string(),
                current_supply,
                amount,
                true,
            )
            .unwrap();

            assert_eq!(price, expected_price);
        }

        #[test]
        fn test_calculate_exponential_price_sell() {
            let mut deps = mock_dependencies();

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token".to_string(),
            };

            let token_pair = TokenPair {
                enabled: true,
                base_token: "ubase_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
            };

            let pool = Pool {
                enabled: true,
                token_sold: Uint128::new(100_000),
                pair_id: "pair_id".to_string(),
                curve_slope: Uint128::from(1u128),
                token_address: Addr::unchecked("token_address"),
                total_reserve_token: Uint128::from(1_000u128),
                total_volume: Uint128::from(100_000u128),
                total_trades: Uint128::from(100u128),
                total_fees_collected: Uint128::from(10u128),
                last_price: Uint128::from(5u128),
            };

            let token_info = TokenInfo {
                name: "Test Token".to_owned(),
                symbol: "TST".to_owned(),
                decimals: 9,
                total_supply: 100_000_000_000u128.into(),
                initial_price: BASE_PRICE.into(),
                max_price_impact: Uint128::from(30u128),
                graduated: false,
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();
            TOKEN_PAIRS
                .save(deps.as_mut().storage, "pair_id".to_string(), &token_pair)
                .unwrap();
            POOLS
                .save(deps.as_mut().storage, "token_address".to_string(), &pool)
                .unwrap();
            TOKEN_INFO
                .save(
                    deps.as_mut().storage,
                    "token_address".to_string(),
                    &token_info,
                )
                .unwrap();

            let base_price = Decimal::from_ratio(BASE_PRICE, Uint128::new(1_000_000));
            let slope = Decimal::from_ratio(pool.curve_slope, Uint128::new(1_000_000));

            let current_supply = Uint128::new(100_000);
            let amount = Uint128::new(1_000);
            let lower_bound = current_supply - amount;
            let upper_bound = current_supply;

            let lower_dec = Decimal::from_ratio(lower_bound, Uint128::new(1));
            let upper_dec = Decimal::from_ratio(upper_bound, Uint128::new(1));
            let alpha = Decimal::from_str("0.1").unwrap();

            let exp_upper = calculate_ema_exp(slope * upper_dec, alpha).unwrap();
            let exp_lower = calculate_ema_exp(slope * lower_dec, alpha).unwrap();

            let avg_price = base_price * (exp_upper - exp_lower)
                / (slope * Decimal::from_ratio(amount, Uint128::new(1)));

            let expected_price = Uint128::new(
                (avg_price * Decimal::from_ratio(10_u128.pow(token_info.decimals as u32), 1u128))
                    .to_uint_floor()
                    .into(),
            );

            let price = calculate_exponential_price(
                deps.as_mut().storage,
                "token_address".to_string(),
                current_supply,
                amount,
                false,
            )
            .unwrap();

            assert_eq!(price, expected_price);
        }

        #[test]
        fn test_calculate_exponential_price_supply_exceeds_maximum() {
            let mut deps = mock_dependencies();

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token".to_string(),
            };

            let token_pair = TokenPair {
                enabled: true,
                base_token: "ubase_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
            };

            let pool = Pool {
                enabled: true,
                token_sold: Uint128::new(80_000_000_000u128),
                pair_id: "pair_id".to_string(),
                curve_slope: Uint128::from(1u128),
                token_address: Addr::unchecked("token_address"),
                total_reserve_token: Uint128::from(1_000u128),
                total_volume: Uint128::from(100_000u128),
                total_trades: Uint128::from(100u128),
                total_fees_collected: Uint128::from(10u128),
                last_price: Uint128::from(5u128),
            };

            let token_info = TokenInfo {
                name: "Test Token".to_owned(),
                symbol: "TST".to_owned(),
                decimals: 9,
                total_supply: 100_000_000_000u128.into(),
                initial_price: BASE_PRICE.into(),
                max_price_impact: Uint128::from(30u128),
                graduated: false,
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();
            TOKEN_PAIRS
                .save(deps.as_mut().storage, "pair_id".to_string(), &token_pair)
                .unwrap();
            POOLS
                .save(deps.as_mut().storage, "token_address".to_string(), &pool)
                .unwrap();
            TOKEN_INFO
                .save(
                    deps.as_mut().storage,
                    "token_address".to_string(),
                    &token_info,
                )
                .unwrap();

            let result = calculate_exponential_price(
                deps.as_mut().storage,
                "token_address".to_string(),
                Uint128::new(80_000_000_001u128), // Exceeds maximum supply
                Uint128::new(1_000),
                true,
            );

            match result {
                Err(err) => assert_eq!(
                    err,
                    StdError::generic_err("Supply exceeds maximum limit for pricing")
                ),
                _ => panic!("Expected error"),
            }
        }

        #[test]
        fn test_bonding_curve_swap_slippage_tolerance_exceeded() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let info = message_info(
                &Addr::unchecked("buyer"),
                &[Coin {
                    amount: Uint128::from(1000u128),
                    denom: "ubase_token".to_string(),
                }],
            );

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token".to_string(),
            };

            let token_pair = TokenPair {
                enabled: true,
                base_token: "ubase_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
            };

            let pool = Pool {
                enabled: true,
                token_sold: Uint128::new(100_000),
                pair_id: "pair_id".to_string(),
                curve_slope: Uint128::from(1u128),
                token_address: Addr::unchecked("token_address"),
                total_reserve_token: Uint128::from(1_000u128),
                total_volume: Uint128::from(100_000u128),
                total_trades: Uint128::from(100u128),
                total_fees_collected: Uint128::from(10u128),
                last_price: Uint128::from(5u128),
            };

            let token_info = TokenInfo {
                name: "Test Token".to_owned(),
                symbol: "TST".to_owned(),
                decimals: 9,
                total_supply: 100_000_000_000u128.into(),
                initial_price: BASE_PRICE.into(),
                max_price_impact: Uint128::from(30u128),
                graduated: false,
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();
            TOKEN_PAIRS
                .save(deps.as_mut().storage, "pair_id".to_string(), &token_pair)
                .unwrap();
            POOLS
                .save(deps.as_mut().storage, "token_address".to_string(), &pool)
                .unwrap();
            TOKEN_INFO
                .save(
                    deps.as_mut().storage,
                    "token_address".to_string(),
                    &token_info,
                )
                .unwrap();

            let result = execute_bonding_curve_swap(
                deps.as_mut(),
                env,
                info,
                "pair_id".to_string(),
                "token_address".to_string(),
                Uint128::new(1000),
                Uint128::new(1_000_000_000), // High min_return to trigger slippage error
                OrderType::Buy,
            );

            match result {
                Err(err) => assert_eq!(
                    err,
                    StdError::generic_err(
                        "Slippage tolerance exceeded. Expected: 99, Minimum: 1000000000"
                    )
                ),
                _ => panic!("Expected error"),
            }
        }

        #[test]
        fn test_bonding_curve_swap_insufficient_liquidity() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let info = message_info(
                &Addr::unchecked("buyer"),
                &[Coin {
                    denom: "ubase_token".to_string(),
                    amount: Uint128::from(1000u128),
                }],
            );

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token".to_string(),
            };

            let token_pair = TokenPair {
                enabled: true,
                base_token: "ubase_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
            };

            let pool = Pool {
                enabled: true,
                token_sold: Uint128::new(80_000_000_000u128),
                pair_id: "pair_id".to_string(),
                curve_slope: Uint128::from(1u128),
                token_address: Addr::unchecked("token_address"),
                total_reserve_token: Uint128::from(1u128),
                total_volume: Uint128::from(1000u128),
                total_trades: Uint128::from(100u128),
                total_fees_collected: Uint128::from(1u128),
                last_price: Uint128::from(1u128),
            };

            let token_info = TokenInfo {
                name: "Test Token".to_owned(),
                symbol: "TST".to_owned(),
                decimals: 9,
                total_supply: 100_000_000_000u128.into(),
                initial_price: BASE_PRICE.into(),
                max_price_impact: Uint128::from(30u128),
                graduated: false,
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();
            TOKEN_PAIRS
                .save(deps.as_mut().storage, "pair_id".to_string(), &token_pair)
                .unwrap();
            POOLS
                .save(deps.as_mut().storage, "token_address".to_string(), &pool)
                .unwrap();
            TOKEN_INFO
                .save(
                    deps.as_mut().storage,
                    "token_address".to_string(),
                    &token_info,
                )
                .unwrap();

            let result = execute_bonding_curve_swap(
                deps.as_mut(),
                env,
                info,
                "pair_id".to_string(),
                "token_address".to_string(),
                Uint128::new(1000),
                Uint128::new(1),
                OrderType::Buy,
            );

            match result {
                Err(err) => {
                    assert_eq!(
                        err,
                        StdError::generic_err("Supply exceeds maximum limit for pricing")
                    )
                }
                _ => panic!("Expected error"),
            }
        }

        #[test]
        fn test_bonding_curve_swap_buy_success() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let info = message_info(
                &Addr::unchecked("buyer"),
                &[Coin {
                    denom: "ubase_token".to_string(),
                    amount: Uint128::from(1000u128),
                }],
            );

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token".to_string(),
            };

            let token_pair = TokenPair {
                enabled: true,
                base_token: "ubase_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
            };

            let pool = Pool {
                enabled: true,
                token_sold: Uint128::new(10_000_000_000u128),
                pair_id: "pair_id".to_string(),
                curve_slope: Uint128::from(1u128),
                token_address: Addr::unchecked("token_address"),
                total_reserve_token: Uint128::from(1u128),
                total_volume: Uint128::from(1u128),
                total_trades: Uint128::from(1u128),
                total_fees_collected: Uint128::from(1u128),
                last_price: Uint128::from(1u128),
            };

            let token_info = TokenInfo {
                name: "Test Token".to_owned(),
                symbol: "TST".to_owned(),
                decimals: 9,
                total_supply: 100_000_000_000u128.into(),
                initial_price: BASE_PRICE.into(),
                max_price_impact: Uint128::from(30u128),
                graduated: false,
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();
            TOKEN_PAIRS
                .save(deps.as_mut().storage, "pair_id".to_string(), &token_pair)
                .unwrap();
            POOLS
                .save(deps.as_mut().storage, "token_address".to_string(), &pool)
                .unwrap();
            TOKEN_INFO
                .save(
                    deps.as_mut().storage,
                    "token_address".to_string(),
                    &token_info,
                )
                .unwrap();

            // Mock the calculate_exponential_price function to return a known price
            let amount = Uint128::new(1000);
            let price = calculate_exponential_price(
                &deps.storage,
                "token_address".to_owned(),
                10_000_000_000u128.into(),
                amount,
                true,
            )
            .unwrap();

            let tokens_to_receive = amount * price / Uint128::new(1_000_000);

            // Execute the bonding curve swap function
            let response = execute_bonding_curve_swap(
                deps.as_mut(),
                env,
                info,
                "pair_id".to_string(),
                "token_address".to_string(),
                amount,
                Uint128::new(1), // min_return
                OrderType::Buy,
            )
            .unwrap();

            // Verify the response attributes and messages
            assert_eq!(response.attributes.len(), 6);
            assert_eq!(response.attributes[0], attr("action", "bonding_curve_swap"));
            assert_eq!(response.attributes[1], attr("pair_id", "pair_id"));
            assert_eq!(response.attributes[2], attr("order_type", "Buy"));
            assert_eq!(response.attributes[3], attr("base_amount", amount));
            assert_eq!(
                response.attributes[4],
                attr("quote_amount", tokens_to_receive)
            );
            assert_eq!(response.attributes[5], attr("price", price.to_string()));

            // Verify messages include transfer of tokens
            assert_eq!(response.messages.len(), 1); // 1 transfer message for Buy
        }

        #[test]
        fn test_match_limit_orders_fully_matched() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let info = message_info(&Addr::unchecked("buyer"), &[]);

            // Initialize the token pair and config
            let token_pair = TokenPair {
                base_token: "ubase_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
                enabled: true,
            };

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token".to_string(),
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();
            TOKEN_PAIRS
                .save(deps.as_mut().storage, "pair_id".to_string(), &token_pair)
                .unwrap();

            // Initialize the order book
            let mut order_book = OrderBook {
                pair_id: "pair_id".to_string(),
                buy_orders: BTreeMap::new(),
                sell_orders: BTreeMap::new(),
            };

            let sell_order = Order {
                id: 1,
                owner: Addr::unchecked("seller"),
                pair_id: "pair_id".to_string(),
                token_amount: Uint128::new(1000),
                price: Uint128::new(1),
                timestamp: env.block.time.seconds() as u64,
                status: OrderStatus::Active,
                filled_amount: Uint128::zero(),
                remaining_amount: Uint128::new(1000),
                order_type: OrderType::Sell,
                created_at: env.block.height,
            };

            order_book.sell_orders.insert(1, vec![sell_order.clone()]);
            ORDER_BOOKS
                .save(deps.as_mut().storage, "pair_id".to_string(), &order_book)
                .unwrap();

            // Initialize NEXT_TRADE_ID
            NEXT_TRADE_ID.save(deps.as_mut().storage, &1).unwrap();

            // Match limit orders
            let (matched_amount, remaining_amount, response) = match_limit_orders(
                deps.as_mut().storage,
                &info,
                &env,
                "pair_id".to_string(),
                Uint128::new(1000), // Buy order amount
                &OrderType::Buy,
                Uint128::zero(),
            )
            .unwrap();

            // Calculate expected fees
            let total_price = Uint128::new(1000) * Uint128::new(1); // amount * price
            let expected_maker_fee = (Decimal::new(total_price) * config.maker_fee)
                .checked_div(Decimal::percent(100))
                .unwrap();
            let expected_taker_fee = (Decimal::new(total_price) * config.taker_fee)
                .checked_div(Decimal::percent(100))
                .unwrap();

            // Verify matched and remaining amounts
            assert_eq!(matched_amount, Uint128::new(1000));
            assert_eq!(remaining_amount, Uint128::zero());

            // Verify response attributes
            assert_eq!(response.attributes.len(), 15);
            assert_eq!(response.attributes[0], attr("event_type", "trade"));
            assert_eq!(response.attributes[1], attr("trade_id", "1"));
            assert_eq!(response.attributes[2], attr("pair_id", "pair_id"));
            assert_eq!(response.attributes[3], attr("buy_order_id", "market_order"));
            assert_eq!(response.attributes[4], attr("sell_order_id", "1"));
            assert_eq!(response.attributes[5], attr("price", "1"));
            assert_eq!(response.attributes[6], attr("amount", "1000"));
            assert_eq!(response.attributes[7], attr("total", "1000"));
            assert_eq!(
                response.attributes[8],
                attr("maker_fee", expected_maker_fee.to_string())
            );
            assert_eq!(
                response.attributes[9],
                attr("taker_fee", expected_taker_fee.to_string())
            );
            assert_eq!(response.attributes[10], attr("base_token", "ubase_token"));
            assert_eq!(response.attributes[11], attr("quote_token", "quote_token"));
            assert_eq!(
                response.attributes[12],
                attr("timestamp", env.block.time.seconds().to_string())
            );
            assert_eq!(response.attributes[13], attr("matched_amount", "1000"));
            assert_eq!(response.attributes[14], attr("remaining_amount", "0"));

            // Verify that the sell order is removed from the order book
            let order_book: OrderBook = ORDER_BOOKS
                .load(deps.as_mut().storage, "pair_id".to_string())
                .unwrap();
            assert!(order_book.sell_orders.get(&1).is_none());
        }

        #[test]
        fn test_match_limit_orders_partially_matched() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let info = message_info(&Addr::unchecked("buyer"), &[]);

            // Initialize the token pair and config
            let token_pair = TokenPair {
                base_token: "ubase_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
                enabled: true,
            };

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token".to_string(),
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();
            TOKEN_PAIRS
                .save(deps.as_mut().storage, "pair_id".to_string(), &token_pair)
                .unwrap();

            // Initialize the order book
            let mut order_book = OrderBook {
                pair_id: "pair_id".to_string(),
                buy_orders: BTreeMap::new(),
                sell_orders: BTreeMap::new(),
            };

            let sell_order = Order {
                id: 1,
                owner: Addr::unchecked("seller"),
                pair_id: "pair_id".to_string(),
                token_amount: Uint128::new(1000),
                price: Uint128::new(1),
                timestamp: env.block.time.seconds() as u64,
                status: OrderStatus::Active,
                filled_amount: Uint128::zero(),
                remaining_amount: Uint128::new(1000),
                order_type: OrderType::Sell,
                created_at: env.block.height,
            };

            order_book.sell_orders.insert(1, vec![sell_order.clone()]);
            ORDER_BOOKS
                .save(deps.as_mut().storage, "pair_id".to_string(), &order_book)
                .unwrap();

            // Initialize NEXT_TRADE_ID
            NEXT_TRADE_ID.save(deps.as_mut().storage, &1).unwrap();

            // Match limit orders
            let (matched_amount, remaining_amount, response) = match_limit_orders(
                deps.as_mut().storage,
                &info,
                &env,
                "pair_id".to_string(),
                Uint128::new(500), // Buy order amount
                &OrderType::Buy,
                Uint128::zero(),
            )
            .unwrap();

            // Calculate expected fees
            let total_price = Uint128::new(500) * Uint128::new(1); // amount * price
            let expected_maker_fee = (Decimal::new(total_price) * config.maker_fee)
                .checked_div(Decimal::percent(100))
                .unwrap();
            let expected_taker_fee = (Decimal::new(total_price) * config.taker_fee)
                .checked_div(Decimal::percent(100))
                .unwrap();

            // Verify matched and remaining amounts
            assert_eq!(matched_amount, Uint128::new(500));
            assert_eq!(remaining_amount, Uint128::zero());

            // Verify response attributes
            assert_eq!(response.attributes.len(), 15);
            assert_eq!(response.attributes[0], attr("event_type", "trade"));
            assert_eq!(response.attributes[1], attr("trade_id", "1"));
            assert_eq!(response.attributes[2], attr("pair_id", "pair_id"));
            assert_eq!(response.attributes[3], attr("buy_order_id", "market_order"));
            assert_eq!(response.attributes[4], attr("sell_order_id", "1"));
            assert_eq!(response.attributes[5], attr("price", "1"));
            assert_eq!(response.attributes[6], attr("amount", "500"));
            assert_eq!(response.attributes[7], attr("total", "500"));
            assert_eq!(
                response.attributes[8],
                attr("maker_fee", expected_maker_fee.to_string())
            );
            assert_eq!(
                response.attributes[9],
                attr("taker_fee", expected_taker_fee.to_string())
            );
            assert_eq!(response.attributes[10], attr("base_token", "ubase_token"));
            assert_eq!(response.attributes[11], attr("quote_token", "quote_token"));
            assert_eq!(
                response.attributes[12],
                attr("timestamp", env.block.time.seconds().to_string())
            );
            assert_eq!(response.attributes[13], attr("matched_amount", "500"));
            assert_eq!(response.attributes[14], attr("remaining_amount", "0"));

            // Verify that the sell order is partially filled and still in the order book
            let order_book: OrderBook = ORDER_BOOKS
                .load(deps.as_mut().storage, "pair_id".to_string())
                .unwrap();
            let remaining_orders = order_book.sell_orders.get(&1).unwrap();
            assert_eq!(remaining_orders.len(), 1);
            assert_eq!(remaining_orders[0].remaining_amount, Uint128::new(500));
            assert_eq!(remaining_orders[0].filled_amount, Uint128::new(500));
            assert_eq!(remaining_orders[0].status, OrderStatus::Active);
        }

        #[test]
        fn test_match_limit_orders_no_match() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let info = message_info(&Addr::unchecked("buyer"), &[]);

            // Initialize the token pair and config
            let token_pair = TokenPair {
                base_token: "ubase_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
                enabled: true,
            };

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token".to_string(),
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();
            TOKEN_PAIRS
                .save(deps.as_mut().storage, "pair_id".to_string(), &token_pair)
                .unwrap();

            // Initialize the order book with non-matching orders
            let mut order_book = OrderBook {
                pair_id: "pair_id".to_string(),
                buy_orders: BTreeMap::new(),
                sell_orders: BTreeMap::new(),
            };

            let sell_order = Order {
                id: 1,
                owner: Addr::unchecked("seller"),
                pair_id: "pair_id".to_string(),
                token_amount: Uint128::new(10),
                price: Uint128::new(1000), // Higher price to ensure no match
                timestamp: env.block.time.seconds() as u64,
                status: OrderStatus::Active,
                filled_amount: Uint128::zero(),
                remaining_amount: Uint128::new(1000),
                order_type: OrderType::Sell,
                created_at: env.block.height,
            };

            order_book.sell_orders.insert(2, vec![sell_order.clone()]);
            ORDER_BOOKS
                .save(deps.as_mut().storage, "pair_id".to_string(), &order_book)
                .unwrap();

            // Initialize NEXT_TRADE_ID
            NEXT_TRADE_ID.save(deps.as_mut().storage, &1).unwrap();

            // Match limit orders
            let (matched_amount, remaining_amount, response) = match_limit_orders(
                deps.as_mut().storage,
                &info,
                &env,
                "pair_id".to_string(),
                Uint128::new(1000), // Buy order amount
                &OrderType::Buy,
                Uint128::new(100_000_000_000u128),
            )
            .unwrap();

            // Verify matched and remaining amounts
            assert_eq!(matched_amount, Uint128::zero());
            assert_eq!(remaining_amount, Uint128::new(1000));

            // Verify response attributes
            assert_eq!(response.attributes.len(), 2);
            assert_eq!(response.attributes[0], attr("matched_amount", "0"));
            assert_eq!(response.attributes[1], attr("remaining_amount", "1000"));

            // Verify that the non-matching order is still in the order book
            let order_book: OrderBook = ORDER_BOOKS
                .load(deps.as_mut().storage, "pair_id".to_string())
                .unwrap();
            let remaining_orders = order_book.sell_orders.get(&2).unwrap();
            assert_eq!(remaining_orders.len(), 1);
            assert_eq!(remaining_orders[0].remaining_amount, Uint128::new(1000));
            assert_eq!(remaining_orders[0].filled_amount, Uint128::zero());
            assert_eq!(remaining_orders[0].status, OrderStatus::Active);
        }

        #[test]
        fn test_execute_swap_fully_matched() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let info = message_info(&Addr::unchecked("buyer"), &[]);

            // Initialize the token pair and config
            let token_pair = TokenPair {
                base_token: "ubase_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
                enabled: true,
            };

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token".to_string(),
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();
            TOKEN_PAIRS
                .save(deps.as_mut().storage, "pair_id".to_string(), &token_pair)
                .unwrap();

            // Initialize the order book
            let mut order_book = OrderBook {
                pair_id: "pair_id".to_string(),
                buy_orders: BTreeMap::new(),
                sell_orders: BTreeMap::new(),
            };

            let sell_order = Order {
                id: 1,
                owner: Addr::unchecked("seller"),
                pair_id: "pair_id".to_string(),
                token_amount: Uint128::new(1000),
                price: Uint128::new(1),
                timestamp: env.block.time.seconds() as u64,
                status: OrderStatus::Active,
                filled_amount: Uint128::zero(),
                remaining_amount: Uint128::new(1000),
                order_type: OrderType::Sell,
                created_at: env.block.height,
            };

            order_book.sell_orders.insert(1, vec![sell_order.clone()]);
            ORDER_BOOKS
                .save(deps.as_mut().storage, "pair_id".to_string(), &order_book)
                .unwrap();

            // Initialize NEXT_TRADE_ID
            NEXT_TRADE_ID.save(deps.as_mut().storage, &1).unwrap();

            // Execute swap
            let response = execute_swap(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                "pair_id".to_string(),
                "token_address".to_string(),
                Uint128::new(1000),
                Uint128::new(1000), // min_return
                OrderType::Buy,
            )
            .unwrap();

            // Calculate expected fees
            let total_price = Uint128::new(1000) * Uint128::new(1); // amount * price
            let expected_maker_fee = (Decimal::new(total_price) * config.maker_fee)
                .checked_div(Decimal::percent(100))
                .unwrap();
            let expected_taker_fee = (Decimal::new(total_price) * config.taker_fee)
                .checked_div(Decimal::percent(100))
                .unwrap();

            // Verify response attributes
            assert_eq!(response.attributes.len(), 17);
            assert_eq!(response.attributes[0], attr("event_type", "trade"));
            assert_eq!(response.attributes[1], attr("trade_id", "1"));
            assert_eq!(response.attributes[2], attr("pair_id", "pair_id"));
            assert_eq!(response.attributes[3], attr("buy_order_id", "market_order"));
            assert_eq!(response.attributes[4], attr("sell_order_id", "1"));
            assert_eq!(response.attributes[5], attr("price", "1"));
            assert_eq!(response.attributes[6], attr("amount", "1000"));
            assert_eq!(response.attributes[7], attr("total", "1000"));
            assert_eq!(
                response.attributes[8],
                attr("maker_fee", expected_maker_fee.to_string())
            );
            assert_eq!(
                response.attributes[9],
                attr("taker_fee", expected_taker_fee.to_string())
            );
            assert_eq!(response.attributes[10], attr("base_token", "ubase_token"));
            assert_eq!(response.attributes[11], attr("quote_token", "quote_token"));
            assert_eq!(
                response.attributes[12],
                attr("timestamp", env.block.time.seconds().to_string())
            );
            assert_eq!(response.attributes[13], attr("matched_amount", "1000"));
            assert_eq!(response.attributes[14], attr("remaining_amount", "0"));
            assert_eq!(response.attributes[15], attr("matched_amount", "1000"));
            assert_eq!(response.attributes[16], attr("remaining_amount", "0"));
        }

        #[test]
        fn test_execute_swap_partially_matched() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let info = message_info(
                &Addr::unchecked("buyer"),
                &[Coin {
                    denom: "ubase_token".to_string(),
                    amount: Uint128::new(500),
                }],
            );

            // Initialize the token pair and config
            let token_pair = TokenPair {
                base_token: "ubase_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
                enabled: true,
            };

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token".to_string(),
            };

            let token_address = Addr::unchecked("quote_token");
            let pool = Pool {
                pair_id: "pair_id".to_string(),
                curve_slope: Uint128::one(),
                token_address: token_address.clone(),
                total_reserve_token: Uint128::new(1_000_000u128),
                token_sold: Uint128::one(),
                total_volume: Uint128::one(),
                total_trades: Uint128::one(),
                total_fees_collected: Uint128::zero(),
                last_price: Uint128::zero(),
                enabled: true,
            };

            let token_info = TokenInfo {
                name: "Test Token".to_owned(),
                symbol: "TST".to_owned(),
                decimals: 9,
                total_supply: 100_000_000_000u128.into(),
                initial_price: BASE_PRICE.into(),
                max_price_impact: Uint128::from(30u128),
                graduated: false,
            };

            POOLS
                .save(deps.as_mut().storage, token_address.to_string(), &pool)
                .unwrap();

            CONFIG.save(deps.as_mut().storage, &config).unwrap();
            TOKEN_PAIRS
                .save(deps.as_mut().storage, "pair_id".to_string(), &token_pair)
                .unwrap();
            TOKEN_INFO
                .save(
                    deps.as_mut().storage,
                    token_address.to_string(),
                    &token_info,
                )
                .unwrap();

            // Initialize the order book
            let mut order_book = OrderBook {
                pair_id: "pair_id".to_string(),
                buy_orders: BTreeMap::new(),
                sell_orders: BTreeMap::new(),
            };

            let sell_order = Order {
                id: 1,
                owner: Addr::unchecked("seller"),
                pair_id: "pair_id".to_string(),
                token_amount: Uint128::new(500),
                price: Uint128::new(1),
                timestamp: env.block.time.seconds() as u64,
                status: OrderStatus::Active,
                filled_amount: Uint128::zero(),
                remaining_amount: Uint128::new(500),
                order_type: OrderType::Sell,
                created_at: env.block.height,
            };

            order_book.sell_orders.insert(1, vec![sell_order.clone()]);
            ORDER_BOOKS
                .save(deps.as_mut().storage, "pair_id".to_string(), &order_book)
                .unwrap();

            // Initialize NEXT_TRADE_ID
            NEXT_TRADE_ID.save(deps.as_mut().storage, &1).unwrap();

            // Execute swap
            let response = execute_swap(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                "pair_id".to_string(),
                token_address.into_string(),
                Uint128::new(10),
                Uint128::one(), // min_return
                OrderType::Buy,
            )
            .unwrap();

            // Verify response attributes
            assert_eq!(response.attributes.len(), 17);
            assert_eq!(response.attributes[0], attr("event_type", "trade"));
            assert_eq!(response.attributes[1], attr("trade_id", "1"));
            assert_eq!(response.attributes[2], attr("pair_id", "pair_id"));
            assert_eq!(response.attributes[3], attr("buy_order_id", "market_order"));
            assert_eq!(response.attributes[4], attr("sell_order_id", "1"));
            assert_eq!(response.attributes[5], attr("price", "1"));
            assert_eq!(response.attributes[6], attr("amount", "10"));
            assert_eq!(response.attributes[7], attr("total", "10"));

            // Verify order book updates
            let updated_order_book: OrderBook = ORDER_BOOKS
                .load(deps.as_mut().storage, "pair_id".to_string())
                .unwrap();
            let updated_sell_orders = updated_order_book.sell_orders.get(&1).unwrap();
            assert_eq!(updated_sell_orders[0].remaining_amount, Uint128::new(490));
            assert_eq!(updated_sell_orders[0].filled_amount, Uint128::new(10));
            assert_eq!(updated_sell_orders[0].status, OrderStatus::Active); // Order should still be active even if fully filled
        }

        #[test]
        fn test_execute_swap_no_match() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let info = message_info(
                &Addr::unchecked("buyer"),
                &[Coin {
                    denom: "ubase_token".to_string(),
                    amount: Uint128::new(1000),
                }],
            );

            // Initialize the token pair and config
            let token_pair = TokenPair {
                base_token: "ubase_token".to_string(),
                quote_token: "quote_token".to_string(),
                base_decimals: 6,
                quote_decimals: 8,
                enabled: true,
            };

            let config = Config {
                owner: Addr::unchecked("creator"),
                token_factory: Addr::unchecked("token_factory_addr"),
                fee_collector: Addr::unchecked("fee_collector_addr"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token".to_string(),
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();
            TOKEN_PAIRS
                .save(deps.as_mut().storage, "pair_id".to_string(), &token_pair)
                .unwrap();

            // Initialize the order book with no matching orders
            let order_book = OrderBook {
                pair_id: "pair_id".to_string(),
                buy_orders: BTreeMap::new(),
                sell_orders: BTreeMap::new(),
            };
            ORDER_BOOKS
                .save(deps.as_mut().storage, "pair_id".to_string(), &order_book)
                .unwrap();

            // Initialize NEXT_TRADE_ID
            NEXT_TRADE_ID.save(deps.as_mut().storage, &1).unwrap();

            // Initialize the pool data
            let token_address = Addr::unchecked("quote_token");
            let pool = Pool {
                pair_id: "pair_id".to_string(),
                curve_slope: Uint128::one(),
                token_address: token_address.clone(),
                total_reserve_token: Uint128::new(1_000_000u128),
                token_sold: Uint128::one(),
                total_volume: Uint128::one(),
                total_trades: Uint128::one(),
                total_fees_collected: Uint128::zero(),
                last_price: Uint128::zero(),
                enabled: true,
            };
            POOLS
                .save(
                    deps.as_mut().storage,
                    token_address.clone().into_string(),
                    &pool,
                )
                .unwrap();

            let token_info = TokenInfo {
                name: "Test Token".to_owned(),
                symbol: "TST".to_owned(),
                decimals: 9,
                total_supply: 100_000_000_000u128.into(),
                initial_price: BASE_PRICE.into(),
                max_price_impact: Uint128::from(30u128),
                graduated: false,
            };

            TOKEN_INFO
                .save(
                    deps.as_mut().storage,
                    token_address.to_string(),
                    &token_info,
                )
                .unwrap();

            // Execute swap
            let response = execute_swap(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                "pair_id".to_string(),
                token_address.into_string(),
                Uint128::new(1000),
                Uint128::new(1000), // min_return
                OrderType::Buy,
            )
            .unwrap();

            // Verify response attributes
            assert_eq!(response.attributes.len(), 10);
            assert_eq!(response.attributes[0], attr("matched_amount", "0"));
            assert_eq!(response.attributes[1], attr("remaining_amount", "1000"));

            // Verify order book updates
            let updated_order_book: OrderBook = ORDER_BOOKS
                .load(deps.as_mut().storage, "pair_id".to_string())
                .unwrap();
            assert!(updated_order_book.buy_orders.is_empty());
            assert!(updated_order_book.sell_orders.is_empty());
        }

        #[test]
        fn test_execute_graduate() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let owner_info = message_info(&Addr::unchecked("owner"), &[]);
            let non_owner_info = message_info(&Addr::unchecked("non_owner"), &[]);

            // Initialize the config
            let config = Config {
                owner: Addr::unchecked("owner"),
                token_factory: Addr::unchecked("initial_factory"),
                fee_collector: Addr::unchecked("initial_collector"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: Uint128::new(100_000_000_000).into(),
                bonding_curve_supply: Uint128::new(80_000_000_000).into(),
                lp_supply: Uint128::new(20_000_000_000).into(),
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token".to_string(),
            };

            CONFIG.save(deps.as_mut().storage, &config).unwrap();

            // Initialize the token info
            let token_info = TokenInfo {
                name: "Test Token".to_owned(),
                symbol: "TST".to_owned(),
                decimals: 9,
                total_supply: 100_000_000_000u128.into(),
                initial_price: BASE_PRICE.into(),
                max_price_impact: Uint128::from(30u128),
                graduated: false,
            };

            let token_address = Addr::unchecked("quote_token");
            TOKEN_INFO
                .save(
                    deps.as_mut().storage,
                    token_address.to_string(),
                    &token_info,
                )
                .unwrap();

            // Initialize the pool
            let pool = Pool {
                pair_id: "pair_id".to_string(),
                curve_slope: Uint128::one(),
                token_address: token_address.clone(),
                total_reserve_token: Uint128::new(1_000_000u128),
                token_sold: Uint128::from(80_000_000_000u128),
                total_volume: Uint128::one(),
                total_trades: Uint128::one(),
                total_fees_collected: Uint128::zero(),
                last_price: Uint128::zero(),
                enabled: true,
            };

            POOLS
                .save(deps.as_mut().storage, token_address.to_string(), &pool)
                .unwrap();

            // Test unauthorized graduation
            let unauthorized_graduation = execute_graduate(
                deps.as_mut(),
                env.clone(),
                non_owner_info.clone(),
                token_address.to_string(),
            );

            assert!(unauthorized_graduation.is_err());
            assert_eq!(
                unauthorized_graduation.err().unwrap(),
                StdError::generic_err("Unauthorized")
            );

            // Test successful graduation
            let successful_graduation = execute_graduate(
                deps.as_mut(),
                env.clone(),
                owner_info.clone(),
                token_address.to_string(),
            )
            .unwrap();

            assert_eq!(successful_graduation.attributes.len(), 3);
            assert_eq!(
                successful_graduation.attributes[0],
                attr("action", "graduate")
            );
            assert_eq!(
                successful_graduation.attributes[1],
                attr("token", token_address.to_string())
            );
            assert_eq!(
                successful_graduation.attributes[2],
                attr("secondary_amm", "secondary_amm_addr")
            );

            // Verify token info updates
            let updated_token_info: TokenInfo = TOKEN_INFO
                .load(deps.as_mut().storage, token_address.to_string())
                .unwrap();
            assert_eq!(updated_token_info.graduated, true);

            // Verify pool removal
            let pool_exists = POOLS
                .may_load(deps.as_mut().storage, token_address.to_string())
                .unwrap();
            assert!(pool_exists.is_none());

            // Expected WasmMsg
            let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_address.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: "secondary_amm_addr".to_string(),
                    amount: Uint128::new(20_000_000_000),
                    expires: None,
                })
                .unwrap(),
                funds: vec![],
            });

            // Verify messages
            assert_eq!(successful_graduation.messages.len(), 1);
            assert_eq!(successful_graduation.messages[0], SubMsg::new(expected_msg));
        }

        #[test]
        fn test_execute_update_config() {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let owner_info = message_info(&Addr::unchecked("owner"), &[]);
            let non_owner_info = message_info(&Addr::unchecked("non_owner"), &[]);

            // Initialize the config
            let initial_config = Config {
                owner: Addr::unchecked("owner"),
                token_factory: Addr::unchecked("initial_factory"),
                fee_collector: Addr::unchecked("initial_collector"),
                trading_fee: Decimal::percent(1),
                enabled: true,
                quote_token_total_supply: 100_000_000_000u128,
                bonding_curve_supply: 80_000_000_000u128,
                lp_supply: 20_000_000_000u128,
                maker_fee: Decimal::percent(1),
                taker_fee: Decimal::percent(1),
                secondary_amm_address: Addr::unchecked("secondary_amm_addr"),
                base_token_denom: "ubase_token".to_string(),
            };

            CONFIG.save(deps.as_mut().storage, &initial_config).unwrap();

            // Test unauthorized update
            let unauthorized_update = execute_update_config(
                deps.as_mut(),
                env.clone(),
                non_owner_info.clone(),
                Some(Addr::unchecked("new_factory")),
                Some(Addr::unchecked("new_collector")),
                Some(Decimal::percent(2)),
                Some(Uint128::new(200_000_000_000)),
                Some(Uint128::new(160_000_000_000)),
                Some(Uint128::new(40_000_000_000)),
                Some(false),
            );

            assert!(unauthorized_update.is_err());
            assert_eq!(
                unauthorized_update.err().unwrap(),
                StdError::generic_err("Unauthorized")
            );

            // Test authorized update
            let authorized_update = execute_update_config(
                deps.as_mut(),
                env.clone(),
                owner_info.clone(),
                Some(Addr::unchecked("new_factory")),
                Some(Addr::unchecked("new_collector")),
                Some(Decimal::percent(2)),
                Some(Uint128::new(200_000_000_000)),
                Some(Uint128::new(160_000_000_000)),
                Some(Uint128::new(40_000_000_000)),
                Some(false),
            )
            .unwrap();

            assert_eq!(authorized_update.attributes.len(), 1);
            assert_eq!(
                authorized_update.attributes[0],
                attr("action", "update_config")
            );

            // Verify config updates
            let updated_config: Config = CONFIG.load(deps.as_mut().storage).unwrap();
            assert_eq!(updated_config.token_factory, Addr::unchecked("new_factory"));
            assert_eq!(
                updated_config.fee_collector,
                Addr::unchecked("new_collector")
            );
            assert_eq!(updated_config.trading_fee, Decimal::percent(2));
            assert_eq!(
                updated_config.quote_token_total_supply,
                Uint128::new(200_000_000_000).into()
            );
            assert_eq!(
                updated_config.bonding_curve_supply,
                Uint128::new(160_000_000_000).into()
            );
            assert_eq!(
                updated_config.lp_supply,
                Uint128::new(40_000_000_000).into()
            );
            assert_eq!(updated_config.enabled, false);
        }
    }
}

// #[cfg_attr(not(feature = "library"), entry_point)]
// pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
//     match msg {
//         QueryMsg::GetCount {} => to_json_binary(&query::count(deps)?),
//     }
// }

// pub mod query {
//     use super::*;

//     pub fn count(deps: Deps) -> StdResult<GetCountResponse> {
//         let state = STATE.load(deps.storage)?;
//         Ok(GetCountResponse { count: state.count })
//     }
// }
