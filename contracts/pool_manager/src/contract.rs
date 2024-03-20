use cosmwasm_std::{
    entry_point, from_binary, to_binary, wasm_execute, Addr, Api, Binary, Decimal, Deps, DepsMut,
    Env, MessageInfo, Reply, Response, StdError, StdResult, SubMsg, SubMsgResult, Uint128,
};
use cw2::{get_contract_version, set_contract_version};


use astroport::asset::{addr_opt_validate, Asset, AssetInfo};
use astroport::pair::{QueryMsg as PairQueryMsg, SimulationResponse};
use astroport::querier::query_pair_info;
use astroport::router::{
    ConfigResponse, Cw20HookMsg,InstantiateMsg, MigrateMsg, QueryMsg,
    SimulateSwapOperationsResponse, SwapOperation, SwapResponseData, MAX_SWAP_OPERATIONS,
};
use crate::msg::ExecuteMsg;
use crate::error::ContractError;
use crate::handlers::{execute_swap_operations,execute_create_pair,execute_provide_liquidity,execute_withdraw_liquidity};
use astroport_pcl_common::state::{
    AmpGamma, Config, PoolParams, PoolState, Precisions, PriceState,
};
use crate::query::simulate_swap_operations;
use crate::state::{ PAIR_BALANCES};

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
         
        ExecuteMsg::CreatePairMsg{asset_infos,token_code_id,init_params}=>execute_create_pair(deps, env, info,init_params,asset_infos),
        
        ExecuteMsg::ProvideLiquidity{assets,slippage_tolerance,auto_stake,receiver}=>execute_provide_liquidity(deps, env, info,assets,slippage_tolerance,auto_stake,receiver),
        ExecuteMsg::WithdrawLiquidity{assets,amount}=>execute_withdraw_liquidity(deps,env,info,info.sender,amount,assets),
    }  
}



/**#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg {
        Reply {
            id: INSTANTIATE_TOKEN_REPLY_ID,
            result:
                SubMsgResult::Ok(SubMsgResponse {
                    data: Some(data), ..
                }),
        } => {
            let mut config = CONFIG.load(deps.storage)?;

            if config.pair_info.liquidity_token != Addr::unchecked("") {
                return Err(ContractError::Unauthorized {});
            }

            let init_response = parse_instantiate_response_data(data.as_slice())
                .map_err(|e| StdError::generic_err(format!("{e}")))?;
            config.pair_info.liquidity_token =
                deps.api.addr_validate(&init_response.contract_address)?;
            CONFIG.save(deps.storage, &config)?;
            Ok(Response::new()
                .add_attribute("liquidity_token_addr", config.pair_info.liquidity_token))
        }
        _ => Err(ContractError::FailedToParseReply {}),
    }
}
**/

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

