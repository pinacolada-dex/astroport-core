use cosmwasm_std::{
    entry_point, from_binary, to_binary, wasm_execute, Addr, Api, Binary, Decimal, Deps, DepsMut,
    Env, MessageInfo, Reply, Response, StdError, StdResult, SubMsg, SubMsgResult, Uint128,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::Cw20ReceiveMsg;

use astroport::asset::{addr_opt_validate, Asset, AssetInfo};
use astroport::pair::{QueryMsg as PairQueryMsg, SimulationResponse};
use astroport::querier::query_pair_info;
use astroport::router::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
    SimulateSwapOperationsResponse, SwapOperation, SwapResponseData, MAX_SWAP_OPERATIONS,
};

use crate::error::ContractError;
use crate::handlers::execute_swap_operations;
use crate::operations::execute_swap_operation;
use crate::state::{Config, ReplyData, CONFIG, REPLY_DATA,PAIR_BALANCES};
use crate::msg::ExecuteMsg;
/// Contract name that is used for migration.
const CONTRACT_NAME: &str = "pina-colada";
/// Contract version that is used for migration.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const AFTER_SWAP_REPLY_ID: u64 = 1;

/// Creates a new contract with the specified parameters in the [`InstantiateMsg`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CONFIG.save(
        deps.storage,
        &Config {
            astroport_factory: deps.api.addr_validate(&msg.astroport_factory)?,
        },
    )?;

    Ok(Response::default())
}

/// Exposes all the execute functions available in the contract.
///
/// ## Variants
/// * **ExecuteMsg::Receive(msg)** Receives a message of type [`Cw20ReceiveMsg`] and processes
/// it depending on the received template.
///
/// * **ExecuteMsg::ExecuteSwapOperations {
///             operations,
///             minimum_receive,
///             to
///         }** Performs swap operations with the specified parameters.
///
/// * **ExecuteMsg::ExecuteSwapOperation { operation, to }** Execute a single swap operation.
///
/// * **ExecuteMsg::AssertMinimumReceive {
///             asset_info,
///             prev_balance,
///             minimum_receive,
///             receiver
///         }** Checks if an ask amount is higher than or equal to the minimum amount to receive.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        
        ExecuteMsg::ExecuteSwapOperations {
            operations,
            minimum_receive,
            to,
            max_spread,
        } => execute_swap_operations(
            deps,
            env,
            info.sender,
            operations,
            minimum_receive,
            to,
            max_spread,
        ),
        ExecuteMsg::ExecuteSwapOperation {
            operation,
            to,
            max_spread,
            single,
        } => execute_swap_operation(deps, env, info, operation, to, max_spread, single),        
         
        ExecuteMsg::CreatePairMsg{asset_infos,token_code_id,init_params}=>execute_create_pair(deps, env, info,asset_infos,token_code_id,init_params),
        
        ExecuteMsg::ProvideLiquidity{assets_infos,slippage_tolerance,auto_stake,receiver}=>execute_provide_liquidity(deps, env, info,asset_infos,slippage_tolerance,auto_stake,receiver),
        //ExecuteMsg::WithdrawLiquidity()=execute_withdraw_liquidity()
    }  
}


/// Performs swap operations with the specified parameters.
///
/// * **sender** address that swaps tokens.
///
/// * **operations** all swap operations to perform.
///
/// * **minimum_receive** used to guarantee that the ask amount is above a minimum amount.
///
/// * **to** recipient of the ask tokens.


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg {
        Reply {
            id: AFTER_SWAP_REPLY_ID,
            result: SubMsgResult::Ok(..),
        } => {
            let reply_data = REPLY_DATA.load(deps.storage)?;
            let receiver_balance = reply_data
                .asset_info
                .query_pool(&deps.querier, reply_data.receiver)?;
            let swap_amount = receiver_balance.checked_sub(reply_data.prev_balance)?;

            if let Some(minimum_receive) = reply_data.minimum_receive {
                if swap_amount < minimum_receive {
                    return Err(ContractError::AssertionMinimumReceive {
                        receive: minimum_receive,
                        amount: swap_amount,
                    });
                }
            }

            // Reply data makes sense ONLY if the first token in multi-hop swap is native.
            let data = to_binary(&SwapResponseData {
                return_amount: swap_amount,
            })?;

            Ok(Response::new().set_data(data))
        }
        _ => Err(StdError::generic_err("Failed to process reply").into()),
    }
}

/// Exposes all the queries available in the contract.
/// ## Queries
/// * **QueryMsg::Config {}** Returns general router parameters using a [`ConfigResponse`] object.
/// * **QueryMsg::SimulateSwapOperations {
///             offer_amount,
///             operations,
///         }** Simulates one or multiple swap operations and returns the end result in a [`SimulateSwapOperationsResponse`] object.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Config {} => Ok(to_binary(&query_config(deps)?)?),
        QueryMsg::SimulateSwapOperations {
            offer_amount,
            operations,
        } => Ok(to_binary(&simulate_swap_operations(
            deps,
            offer_amount,
            operations,
        )?)?),
    }
}

/// Returns general contract settings in a [`ConfigResponse`] object.
pub fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let state = CONFIG.load(deps.storage)?;
    let resp = ConfigResponse {
        astroport_factory: state.astroport_factory.into_string(),
    };

    Ok(resp)
}

/// Manages contract migration.
#[cfg(not(tarpaulin_include))]
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let contract_version = get_contract_version(deps.storage)?;

    match contract_version.contract.as_ref() {
        "router" => match contract_version.version.as_ref() {
            "1.1.1" => {}
            _ => return Err(ContractError::MigrationError {}),
        },
        _ => return Err(ContractError::MigrationError {}),
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("previous_contract_name", &contract_version.contract)
        .add_attribute("previous_contract_version", &contract_version.version)
        .add_attribute("new_contract_name", CONTRACT_NAME)
        .add_attribute("new_contract_version", CONTRACT_VERSION))
}

