use cosmwasm_schema::{cw_serde, QueryResponses};
use astroport::{asset::{Asset, AssetInfo}, router::SimulateSwapOperationsResponse};
use cosmwasm_std::{Decimal, Uint128,Binary};
use astroport::router::{SwapOperation};
use cw20::Cw20ReceiveMsg;



pub const MAX_SWAP_OPERATIONS: usize = 50;

/// This structure holds the parameters used for creating a contract.



/**impl SwapOperation {
    pub fn get_target_asset_info(&self) -> AssetInfo {
        match self {
            SwapOperation::NativeSwap { ask_denom, .. } => AssetInfo::NativeToken {
                denom: ask_denom.clone(),
            },
            SwapOperation::ColadaSwap { ask_asset_info, .. } => ask_asset_info.clone(),
        }
    }
}
**/
/// This structure describes the execute messages available in the contract.
#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    /// ExecuteSwapOperations processes multiple swaps while mentioning the minimum amount of tokens to receive for the last swap operation
    ExecuteSwapOperations {
        operations: Vec<SwapOperation>,
        minimum_receive: Option<Uint128>,
        to: Option<String>,
        max_spread: Option<Decimal>,
    },

    /// Internal use
    /// ExecuteSwapOperation executes a single swap operation
   
    ProvideLiquidity {
        /// The assets available in the pool
        assets: Vec<Asset>,
        /// The slippage tolerance that allows liquidity provision only if the price in the pool doesn't move too much
        slippage_tolerance: Option<Decimal>,
        /// Determines whether the LP tokens minted for the user is auto_staked in the Generator contract
        auto_stake: Option<bool>,
        /// The receiver of LP tokens
        receiver: Option<String>,
    },
    WithdrawLiquidity{
        assets: Vec<Asset>,
        amount: Uint128,
    },
    CreatePair {
        /// Information about assets in the pool
        asset_infos: Vec<AssetInfo>,
        /// The token contract code ID used for the tokens in the pool
        token_code_id: u64,
        
        /// Optional binary serialised parameters for custom pool types
        init_params: Option<Binary>,
    }
}
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Config returns configuration parameters for the contract using a custom [`ConfigResponse`] structure
   
    /// SimulateSwapOperations simulates multi-hop swap operations
    #[returns(SimulateSwapOperationsResponse)]
    SimulateSwapOperations {
        /// The amount of tokens to swap
        offer_amount: Uint128,
        /// The swap operations to perform, each swap involving a specific pool
        operations: Vec<SwapOperation>,
    },
}
