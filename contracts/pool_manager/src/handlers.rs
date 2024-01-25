use astroport::asset::{Asset, AssetInfo};
use astroport::pair::ExecuteMsg as PairExecuteMsg;
use astroport::querier::{query_balance, query_pair_info, query_token_balance};
use astroport::router::SwapOperation;
use astroport::pair_concentrated::{
    ConcentratedPoolParams, ConcentratedPoolUpdateParams, MigrateMsg, UpdatePoolParams,
};
use astroport_pcl_common::utils::{
    assert_max_spread, assert_slippage_tolerance, before_swap_check, calc_provide_fee,
    check_asset_infos, check_assets, check_cw20_in_pool, check_pair_registered, compute_swap,
    get_share_in_assets, mint_liquidity_token_message,
};
use cosmwasm_std::{
    to_binary, Coin, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, StdResult, WasmMsg,Api
};
use cw20::Cw20ExecuteMsg;

use crate::error::ContractError;
use crate::state::{POOLS,PAIR_BALANCES}
use crate::msg::SwapOperation;

/// Returns the end result of a simulation for one or multiple swap
/// operations using a [`SimulateSwapOperationsResponse`] object.
///
/// * **offer_amount** amount of offer assets being swapped.
///
/// * **operations** is a vector that contains objects of type [`SwapOperation`].
/// These are all the swap operations for which we perform a simulation.
fn simulate_swap_operations(
    deps: Deps,
    offer_amount: Uint128,
    operations: Vec<SwapOperation>,
) -> Result<SimulateSwapOperationsResponse, ContractError> {
    assert_operations(deps.api, &operations)?;

    let config = POOLS.load(deps.storage)?;
    let astroport_factory = config.astroport_factory;
    let mut return_amount = offer_amount;

    for operation in operations.into_iter() {
        match operation {
            SwapOperation::AstroSwap {
                offer_asset_info,
                ask_asset_info,
            } => {
                let pair_info = query_pair_info(
                    &deps.querier,
                    astroport_factory.clone(),
                    &[offer_asset_info.clone(), ask_asset_info.clone()],
                )?;

                let res: SimulationResponse = deps.querier.query_wasm_smart(
                    pair_info.contract_addr,
                    &PairQueryMsg::Simulation {
                        offer_asset: Asset {
                            info: offer_asset_info.clone(),
                            amount: return_amount,
                        },
                        ask_asset_info: Some(ask_asset_info.clone()),
                    },
                )?;

                return_amount = res.return_amount;
            }
            SwapOperation::NativeSwap { .. } => {
                return Err(ContractError::NativeSwapNotSupported {})
            }
        }
    }

    Ok(SimulateSwapOperationsResponse {``
        amount: return_amount,
    })
}

/// Validates swap operations.
///
/// * **operations** is a vector that contains objects of type [`SwapOperation`]. These are all the swap operations we check.
fn assert_operations(api: &dyn Api, operations: &[SwapOperation]) -> Result<(), ContractError> {
    let operations_len = operations.len();
    if operations_len == 0 {
        return Err(ContractError::MustProvideOperations {});
    }

    if operations_len > MAX_SWAP_OPERATIONS {
        return Err(ContractError::SwapLimitExceeded {});
    }

    let mut prev_ask_asset: Option<AssetInfo> = None;

    for operation in operations {
        let (offer_asset, ask_asset) = match operation {
            SwapOperation::AstroSwap {
                offer_asset_info,
                ask_asset_info,
            } => (offer_asset_info.clone(), ask_asset_info.clone()),
            SwapOperation::NativeSwap { .. } => {
                return Err(ContractError::NativeSwapNotSupported {})
            }
        };

        offer_asset.check(api)?;
        ask_asset.check(api)?;

        if offer_asset.equal(&ask_asset) {
            return Err(ContractError::DoublingAssetsPath {
                offer_asset: offer_asset.to_string(),
                ask_asset: ask_asset.to_string(),
            });
        }

        if let Some(prev_ask_asset) = prev_ask_asset {
            if prev_ask_asset != offer_asset {
                return Err(ContractError::InvalidPathOperations {
                    prev_ask_asset: prev_ask_asset.to_string(),
                    next_offer_asset: offer_asset.to_string(),
                    next_ask_asset: ask_asset.to_string(),
                });
            }
        }

        prev_ask_asset = Some(ask_asset);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn execute_provide_liquidity( deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mut assets: Vec<Asset>,
    slippage_tolerance: Option<Decimal>,
    auto_stake: Option<bool>,
    receiver: Option<String>) -> Result<Response, ContractError> {
        let mut config = POOLS.load(deps.storage)?;

    if !check_pair_registered(
        deps.querier,
        &config.factory_addr,
        &config.pair_info.asset_infos,
    )? {
        return Err(ContractError::PairIsNotRegistered {});
    }

    match assets.len() {
        0 => {
            return Err(StdError::generic_err("Nothing to provide").into());
        }
        1 => {
            // Append omitted asset with explicit zero amount
            let (given_ind, _) = config
                .pair_info
                .asset_infos
                .iter()
                .find_position(|pool| pool.equal(&assets[0].info))
                .ok_or_else(|| ContractError::InvalidAsset(assets[0].info.to_string()))?;
            assets.push(Asset {
                info: config.pair_info.asset_infos[1 ^ given_ind].clone(),
                amount: Uint128::zero(),
            });
        }
        2 => {}
        _ => {
            return Err(ContractError::InvalidNumberOfAssets(
                config.pair_info.asset_infos.len(),
            ))
        }
    }

    check_assets(deps.api, &assets)?;

    info.funds
        .assert_coins_properly_sent(&assets, &config.pair_info.asset_infos)?;

    let precisions = Precisions::new(deps.storage)?;
    let mut pools = query_pools(deps.querier, &env.contract.address, &config, &precisions)?;

    if pools[0].info.equal(&assets[1].info) {
        assets.swap(0, 1);
    }

    // precisions.get_precision() also validates that the asset belongs to the pool
    let deposits = [
        Decimal256::with_precision(assets[0].amount, precisions.get_precision(&assets[0].info)?)?,
        Decimal256::with_precision(assets[1].amount, precisions.get_precision(&assets[1].info)?)?,
    ];

    let total_share = query_supply(&deps.querier, &config.pair_info.liquidity_token)?
        .to_decimal256(LP_TOKEN_PRECISION)?;

    // Initial provide can not be one-sided
    if total_share.is_zero() && (deposits[0].is_zero() || deposits[1].is_zero()) {
        return Err(ContractError::InvalidZeroAmount {});
    }

    let mut messages = vec![];
    for (i, pool) in pools.iter_mut().enumerate() {
        // If the asset is a token contract, then we need to execute a TransferFrom msg to receive assets
        match &pool.info {
            AssetInfo::Token { contract_addr } => {
                if !deposits[i].is_zero() {
                    messages.push(CosmosMsg::Wasm(wasm_execute(
                        contract_addr,
                        &Cw20ExecuteMsg::TransferFrom {
                            owner: info.sender.to_string(),
                            recipient: env.contract.address.to_string(),
                            amount: deposits[i]
                                .to_uint(precisions.get_precision(&assets[i].info)?)?,
                        },
                        vec![],
                    )?))
                }
            }
            AssetInfo::NativeToken { .. } => {
                // If the asset is native token, the pool balance is already increased
                // To calculate the total amount of deposits properly, we should subtract the user deposit from the pool
                pool.amount = pool.amount.checked_sub(deposits[i])?;
            }
        }
    }

    let mut new_xp = pools
        .iter()
        .enumerate()
        .map(|(ind, pool)| pool.amount + deposits[ind])
        .collect_vec();
    new_xp[1] *= config.pool_state.price_state.price_scale;

    let amp_gamma = config.pool_state.get_amp_gamma(&env);
    let new_d = calc_d(&new_xp, &amp_gamma)?;

    let share = if total_share.is_zero() {
        let xcp = get_xcp(new_d, config.pool_state.price_state.price_scale);
        let mint_amount = xcp
            .checked_sub(MINIMUM_LIQUIDITY_AMOUNT.to_decimal256(LP_TOKEN_PRECISION)?)
            .map_err(|_| ContractError::MinimumLiquidityAmountError {})?;

        messages.extend(mint_liquidity_token_message(
            deps.querier,
            &config,
            &env.contract.address,
            &env.contract.address,
            MINIMUM_LIQUIDITY_AMOUNT,
            false,
        )?);

        // share cannot become zero after minimum liquidity subtraction
        if mint_amount.is_zero() {
            return Err(ContractError::MinimumLiquidityAmountError {});
        }

        config.pool_state.price_state.xcp_profit_real = Decimal256::one();
        config.pool_state.price_state.xcp_profit = Decimal256::one();

        mint_amount
    } else {
        let mut old_xp = pools.iter().map(|a| a.amount).collect_vec();
        old_xp[1] *= config.pool_state.price_state.price_scale;
        let old_d = calc_d(&old_xp, &amp_gamma)?;
        let share = (total_share * new_d / old_d).saturating_sub(total_share);

        let mut ideposits = deposits;
        ideposits[1] *= config.pool_state.price_state.price_scale;

        share * (Decimal256::one() - calc_provide_fee(&ideposits, &new_xp, &config.pool_params))
    };

    // calculate accrued share
    let share_ratio = share / (total_share + share);
    let balanced_share = vec![
        new_xp[0] * share_ratio,
        new_xp[1] * share_ratio / config.pool_state.price_state.price_scale,
    ];
    let assets_diff = vec![
        deposits[0].diff(balanced_share[0]),
        deposits[1].diff(balanced_share[1]),
    ];

    let mut slippage = Decimal256::zero();

    // If deposit doesn't diverge too much from the balanced share, we don't update the price
    if assets_diff[0] >= MIN_TRADE_SIZE && assets_diff[1] >= MIN_TRADE_SIZE {
        slippage = assert_slippage_tolerance(
            &deposits,
            share,
            &config.pool_state.price_state,
            slippage_tolerance,
        )?;

        let last_price = assets_diff[0] / assets_diff[1];
        config.pool_state.update_price(
            &config.pool_params,
            &env,
            total_share + share,
            &new_xp,
            last_price,
        )?;
    }

    let share_uint128 = share.to_uint(LP_TOKEN_PRECISION)?;

    // Mint LP tokens for the sender or for the receiver (if set)
    let receiver = addr_opt_validate(deps.api, &receiver)?.unwrap_or_else(|| info.sender.clone());
    let auto_stake = auto_stake.unwrap_or(false);
    messages.extend(mint_liquidity_token_message(
        deps.querier,
        &config,
        &env.contract.address,
        &receiver,
        share_uint128,
        auto_stake,
    )?);

    if config.track_asset_balances {
        for (i, pool) in pools.iter().enumerate() {
            BALANCES.save(
                deps.storage,
                &pool.info,
                &pool
                    .amount
                    .checked_add(deposits[i])?
                    .to_uint(precisions.get_precision(&pool.info)?)?,
                env.block.height,
            )?;
        }
    }

    POOLS.save(deps.storage, &config)?;

    let attrs = vec![
        attr("action", "provide_liquidity"),
        attr("sender", info.sender),
        attr("receiver", receiver),
        attr("assets", format!("{}, {}", &assets[0], &assets[1])),
        attr("share", share_uint128),
        attr("slippage", slippage.to_string()),
    ];

    Ok(Response::new().add_messages(messages).add_attributes(attrs))

}
pub fn check_asset_infos(api: &dyn Api, asset_infos: &[AssetInfo]) -> Result<(), PclError> {
    if !asset_infos.iter().all_unique() {
        return Err(PclError::DoublingAssets {});
    }

    asset_infos
        .iter()
        .try_for_each(|asset_info| asset_info.check(api))
        .map_err(Into::into)
}
#[allow(clippy::too_many_arguments)]
pub fn execute_create_pair(init_params:Vec<u8>,asset_infos:Vec<AssetInfo>) -> Result<Response, ContractError> {
    if asset_infos.len() != 2 {
        return Err(StdError::generic_err("asset_infos must contain exactly two elements").into());
    }

    check_asset_infos(deps.api, asset_infos)?;

    let params: ConcentratedPoolParams = from_binary(
        init_params
            .ok_or(ContractError::InitParamsNotFound {})?,
    )?;

    if params.price_scale.is_zero() {
        return Err(StdError::generic_err("Initial price scale can not be zero").into());
    }

    

   

    let mut pool_params = PoolParams::default();
    pool_params.update_params(UpdatePoolParams {
        mid_fee: Some(params.mid_fee),
        out_fee: Some(params.out_fee),
        fee_gamma: Some(params.fee_gamma),
        repeg_profit_threshold: Some(params.repeg_profit_threshold),
        min_price_scale_delta: Some(params.min_price_scale_delta),
        ma_half_time: Some(params.ma_half_time),
    })?;

    let pool_state = PoolState {
        initial: AmpGamma::default(),
        future: AmpGamma::new(params.amp, params.gamma)?,
        future_time: env.block.time.seconds(),
        initial_time: 0,
        price_state: PriceState {
            oracle_price: params.price_scale.into(),
            last_price: params.price_scale.into(),
            price_scale: params.price_scale.into(),
            last_price_update: env.block.time.seconds(),
            xcp_profit: Decimal256::zero(),
            xcp_profit_real: Decimal256::zero(),
        },
    };

    let config = Config {
        pair_info: PairInfo {
            contract_addr: env.contract.address.clone(),
            liquidity_token: Addr::unchecked(""),
            asset_infos: msg.asset_infos.clone(),
            pair_type: PairType::Custom("concentrated".to_string()),
        },
        factory_addr,
        pool_params,
        pool_state,
        owner: None,
        track_asset_balances: params.track_asset_balances.unwrap_or_default(),
        fee_share: None,
    };

    if config.track_asset_balances {
        for asset in &config.pair_info.asset_infos {
            BALANCES.save(deps.storage, asset, &Uint128::zero(), env.block.height)?;
        }
    }
    let key = config.create_key();
    POOLS.save(deps.storage, key, &config)?;

    BufferManager::init(deps.storage, OBSERVATIONS, OBSERVATIONS_SIZE)?;

    let token_name = format_lp_token_name(&msg.asset_infos, &deps.querier)?;

    // Create LP token
    let sub_msg = SubMsg::reply_on_success(
        wasm_instantiate(
            msg.token_code_id,
            &TokenInstantiateMsg {
                name: token_name,
                symbol: "uLP".to_string(),
                decimals: LP_TOKEN_PRECISION,
                initial_balances: vec![],
                mint: Some(MinterResponse {
                    minter: env.contract.address.to_string(),
                    cap: None,
                }),
                marketing: None,
            },
            vec![],
            String::from("Pina Colada LP token"),
        )?,
        INSTANTIATE_TOKEN_REPLY_ID,
    );

    Ok(Response::new().add_submessage(sub_msg).add_attribute(
        "asset_balances_tracking".to_owned(),
        if config.track_asset_balances {
            "enabled"
        } else {
            "disabled"
        }
        .to_owned(),
    ))
}

#[allow(clippy::too_many_arguments)]
pub fn execute_swap_operations(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    operations: Vec<SwapOperation>,
    minimum_receive: Option<Uint128>,
    to: Option<String>,
    max_spread: Option<Decimal>,
) -> Result<Response, ContractError> {
    assert_operations(deps.api, &operations)?;

    let to = addr_opt_validate(deps.api, &to)?.unwrap_or(sender);
    let target_asset_info = operations.last().unwrap().get_target_asset_info();
    let operations_len = operations.len();
    /// TODO Replace with internal handler
    /*let messages = operations
        .into_iter()
        .enumerate()
        .map(|(operation_index, op)| {
            if operation_index == operations_len - 1 {
                wasm_execute(
                    env.contract.address.to_string(),
                    &ExecuteMsg::ExecuteSwapOperation {
                        operation: op,
                        to: Some(to.to_string()),
                        max_spread,
                        single: operations_len == 1,
                    },
                    vec![],
                )
                .map(|inner_msg| SubMsg::reply_on_success(inner_msg, AFTER_SWAP_REPLY_ID))
            } else {
                wasm_execute(
                    env.contract.address.to_string(),
                    &ExecuteMsg::ExecuteSwapOperation {
                        operation: op,
                        to: None,
                        max_spread,
                        single: operations_len == 1,
                    },
                    vec![],
                )
                .map(SubMsg::new)
            }
        })
        .collect::<StdResult<Vec<_>>>()?;

    let prev_balance = target_asset_info.query_pool(&deps.querier, &to)?;
    REPLY_DATA.save(
        deps.storage,
        &ReplyData {
            asset_info: target_asset_info,
            prev_balance,
            minimum_receive,
            receiver: to.to_string(),
        },
    )?;
    **/
    Ok(Response::new().add_submessages(messages))
}

pub fn execute_swap_operation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    operation: SwapOperation,
    to: Option<String>,
    max_spread: Option<Decimal>,
    single: bool,
) -> Result<Response, ContractError> {
    if env.contract.address != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let message = match operation {
        SwapOperation::AstroSwap {
            offer_asset_info,
            ask_asset_info,
        } => {
            let config = POOLS.load(deps.storage)?;
            let pair_info = query_pair_info(
                &deps.querier,
                config.astroport_factory,
                &[offer_asset_info.clone(), ask_asset_info.clone()],
            )?;

            let amount = match &offer_asset_info {
                AssetInfo::NativeToken { denom } => {
                    query_balance(&deps.querier, env.contract.address, denom)?
                }
                AssetInfo::Token { contract_addr } => {
                    query_token_balance(&deps.querier, contract_addr, env.contract.address)?
                }
            };
            let offer_asset = Asset {
                info: offer_asset_info,
                amount,
            };

            asset_into_swap_msg(
                pair_info.contract_addr.to_string(),
                offer_asset,
                ask_asset_info,
                max_spread,
                to,
                single,
            )?
        }
        SwapOperation::NativeSwap { .. } => return Err(ContractError::NativeSwapNotSupported {}),
    };

    Ok(Response::new().add_message(message))
}
/// Performs an swap operation with the specified parameters. The trader must approve the
/// pool contract to transfer offer assets from their wallet.
///
/// * **sender** is the sender of the swap operation.
///
/// * **offer_asset** proposed asset for swapping.
///
/// * **belief_price** is used to calculate the maximum swap spread.
///
/// * **max_spread** sets the maximum spread of the swap operation.
///
/// * **to** sets the recipient of the swap operation.
fn swap(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    pool_key:String,
    offer_asset: Asset,
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    to: Option<Addr>,
) -> Result<Response, ContractError> {
    let precisions = Precisions::new(deps.storage)?;
    let offer_asset_prec = precisions.get_precision(&offer_asset.info)?;
    let offer_asset_dec = offer_asset.to_decimal_asset(offer_asset_prec)?;
    let mut config = POOLS.load(deps.storage,pool_key)?;

    //let mut pools = query_pools(deps.querier, &env.contract.address, &config, &precisions)?;

    let (offer_ind, _) = pools
        .iter()
        .find_position(|asset| asset.info == offer_asset_dec.info)
        .ok_or_else(|| ContractError::InvalidAsset(offer_asset_dec.info.to_string()))?;
    let ask_ind = 1 ^ offer_ind;
    let ask_asset_prec = precisions.get_precision(&pools[ask_ind].info)?;

    pools[offer_ind].amount -= offer_asset_dec.amount;

    before_swap_check(&pools, offer_asset_dec.amount)?;

    let mut xs = pools.iter().map(|asset| asset.amount).collect_vec();

    // Get fee info from the factory
    let fee_info = query_fee_info(
        &deps.querier,
        &config.factory_addr,
        config.pair_info.pair_type.clone(),
    )?;
    let mut maker_fee_share = Decimal256::zero();
    if fee_info.fee_address.is_some() {
        maker_fee_share = fee_info.maker_fee_rate.into();
    }
    // If this pool is configured to share fees
    let mut share_fee_share = Decimal256::zero();
    if let Some(fee_share) = config.fee_share.clone() {
        share_fee_share = Decimal256::from_ratio(fee_share.bps, 10000u16);
    }

    let swap_result = compute_swap(
        &xs,
        offer_asset_dec.amount,
        ask_ind,
        &config,
        &env,
        maker_fee_share,
        share_fee_share,
    )?;
    xs[offer_ind] += offer_asset_dec.amount;
    xs[ask_ind] -= swap_result.dy + swap_result.maker_fee + swap_result.share_fee;

    let return_amount = swap_result.dy.to_uint(ask_asset_prec)?;
    let spread_amount = swap_result.spread_fee.to_uint(ask_asset_prec)?;
    assert_max_spread(
        belief_price,
        max_spread,
        offer_asset.amount,
        return_amount,
        spread_amount,
    )?;

    let total_share = query_supply(&deps.querier, &config.pair_info.liquidity_token)?
        .to_decimal256(LP_TOKEN_PRECISION)?;

    // Skip very small trade sizes which could significantly mess up the price due to rounding errors,
    // especially if token precisions are 18.
    if (swap_result.dy + swap_result.maker_fee + swap_result.share_fee) >= MIN_TRADE_SIZE
        && offer_asset_dec.amount >= MIN_TRADE_SIZE
    {
        let last_price = swap_result.calc_last_price(offer_asset_dec.amount, offer_ind);

        // update_price() works only with internal representation
        xs[1] *= config.pool_state.price_state.price_scale;
        config
            .pool_state
            .update_price(&config.pool_params, &env, total_share, &xs, last_price)?;
    }

    let receiver = to.unwrap_or_else(|| sender.clone());

    let mut messages = vec![Asset {
        info: pools[ask_ind].info.clone(),
        amount: return_amount,
    }
    .into_msg(&receiver)?];

    // Send the shared fee
    let mut fee_share_amount = Uint128::zero();
    if let Some(fee_share) = config.fee_share.clone() {
        fee_share_amount = swap_result.share_fee.to_uint(ask_asset_prec)?;
        if !fee_share_amount.is_zero() {
            let fee = pools[ask_ind].info.with_balance(fee_share_amount);
            messages.push(fee.into_msg(fee_share.recipient)?);
        }
    }

    // Send the maker fee
    let mut maker_fee = Uint128::zero();
    if let Some(fee_address) = fee_info.fee_address {
        maker_fee = swap_result.maker_fee.to_uint(ask_asset_prec)?;
        if !maker_fee.is_zero() {
            let fee = pools[ask_ind].info.with_balance(maker_fee);
            messages.push(fee.into_msg(fee_address)?);
        }
    }

    // Store observation from precommit data
    accumulate_swap_sizes(deps.storage, &env)?;

    // Store time series data in precommit observation.
    // Skipping small unsafe values which can seriously mess oracle price due to rounding errors.
    // This data will be reflected in observations in the next action.
    if offer_asset_dec.amount >= MIN_TRADE_SIZE && swap_result.dy >= MIN_TRADE_SIZE {
        let (base_amount, quote_amount) = if offer_ind == 0 {
            (offer_asset.amount, return_amount)
        } else {
            (return_amount, offer_asset.amount)
        };
        PrecommitObservation::save(deps.storage, &env, base_amount, quote_amount)?;
    }
    
    POOLS.save(deps.storage,&config.create_key() &config)?;

    if config.track_asset_balances {
        BALANCES.save(
            deps.storage,
            &pools[offer_ind].info,
            &(pools[offer_ind].amount + offer_asset_dec.amount).to_uint(offer_asset_prec)?,
            env.block.height,
        )?;
        BALANCES.save(
            deps.storage,
            &pools[ask_ind].info,
            &(pools[ask_ind].amount.to_uint(ask_asset_prec)?
                - return_amount
                - maker_fee
                - fee_share_amount),
            env.block.height,
        )?;
    }
}
