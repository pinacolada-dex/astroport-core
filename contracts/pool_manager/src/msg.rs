use cosmwasm_schema::{cw_serde, QueryResponses};
use astroport::asset::AssetInfo;
use cosmwasm_std::{Decimal, Uint128};
use cw20::Cw20ReceiveMsg;

use crate::asset::AssetInfo;

pub const MAX_SWAP_OPERATIONS: usize = 50;

/// This structure holds the parameters used for creating a contract.


/// This enum describes a swap operation.
#[cw_serde]
pub enum SwapOperation {
    /// Native swap
    NativeSwap {
        /// The name (denomination) of the native asset to swap from
        offer_denom: String,
        /// The name (denomination) of the native asset to swap to
        ask_denom: String,
    },
    /// ASTRO swap
    ColadaSwap {
        /// Information about the asset being swapped
        offer_asset_info: AssetInfo,
        /// Information about the asset we swap to
        ask_asset_info: AssetInfo,
    },
}


impl SwapOperation {
    pub fn get_target_asset_info(&self) -> AssetInfo {
        match self {
            SwapOperation::NativeSwap { ask_denom, .. } => AssetInfo::NativeToken {
                denom: ask_denom.clone(),
            },
            SwapOperation::ColadaSwap { ask_asset_info, .. } => ask_asset_info.clone(),
        }
    }
}

/// This structure describes the execute messages available in the contract.
#[cw_serde]
pub enum ExecuteMsg {
  
    /// ExecuteSwapOperations processes multiple swaps while mentioning the minimum amount of tokens to receive for the last swap operation
    ExecuteSwapOperations {
        operations: Vec<SwapOperation>,
        minimum_receive: Option<Uint128>,
        to: Option<String>,
        max_spread: Option<Decimal>,
    },

    /// Internal use
    /// ExecuteSwapOperation executes a single swap operation
    ExecuteSwapOperation {
        operation: SwapOperation,
        to: Option<String>,
        max_spread: Option<Decimal>,
        single: bool,
    },
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
        minimum_receive: Option<Uint128>,
    },
    CreatePairMsg {
        /// Information about assets in the pool
        asset_infos: Vec<AssetInfo>,
        /// The token contract code ID used for the tokens in the pool
        token_code_id: u64,
        
        /// Optional binary serialised parameters for custom pool types
        init_params: Option<Binary>,
    }
}