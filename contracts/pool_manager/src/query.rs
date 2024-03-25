use astroport::router::{SimulateSwapOperationsResponse, SwapOperation};
use cosmwasm_std::{Deps, Uint128};

use crate::error::ContractError;

pub fn simulate_swap_operations(
    _deps: Deps,
    offer_amount: Uint128,
    _operations: Vec<SwapOperation>,
) -> Result<SimulateSwapOperationsResponse, ContractError> {
    //assert_operations(deps.api, &operations)?;


    let return_amount = offer_amount;

    /**for operation in operations.into_iter() {
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
    }**/

    Ok(SimulateSwapOperationsResponse {
        amount: return_amount,
    })
}