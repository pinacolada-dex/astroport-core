use astroport::asset::Asset;
use astroport::cosmwasm_ext::DecimalToInteger;
use astroport::pair::SimulationResponse;
use astroport::router::{SimulateSwapOperationsResponse};
use astroport_pcl_common::utils::compute_swap;
use astroport_pcl_common::state::Precisions;
use astroport_pcl_common::utils::before_swap_check;
use cosmwasm_std::{Decimal256, Deps, DepsMut,Env, Uint128};
use itertools::Itertools;

use crate::error::ContractError;
use crate::handlers::generate_key_from_asset_info;
use crate::msg::SwapOperation;
use crate::state::{ POOLS};
use crate::utils::query_pools;
pub fn simulate_swap_operations(
    deps: DepsMut,
    env:Env,
    offer_amount: Uint128,
    operations: Vec<SwapOperation>,
) -> Result<SimulateSwapOperationsResponse, ContractError> {
    //assert_operations(deps.api, &operations)?;


    let mut return_amount = offer_amount;

    for operation in operations.into_iter() {
        let (offer_asset_info,ask_asset_info)= (operation.offer_asset_info,operation.ask_asset_info);
        let pool_key=generate_key_from_asset_info(&[offer_asset_info.clone(),ask_asset_info.clone()].to_vec());
        let offer_asset=  Asset {
            info: offer_asset_info.clone(),
            amount:return_amount,
        };
        let subresult=query_simulation(deps,env,offer_asset,pool_key).unwrap();
        return_amount=subresult.return_amount;
    }

    Ok(SimulateSwapOperationsResponse {
        amount: return_amount,
    })
}

pub fn query_simulation(
    deps: DepsMut,
    env: Env,
    offer_asset: Asset,
    pool_key:String
) -> Result<SimulationResponse, ContractError> {
    let mut config = POOLS.load(deps.storage,pool_key.clone())?;
    let precisions = Precisions::new(deps.storage)?;
    let offer_asset_prec = precisions.get_precision(&offer_asset.info)?;
    let offer_asset_dec = offer_asset.to_decimal_asset(offer_asset_prec)?;

    let pools = query_pools(&deps, &config, &precisions)?;

    let (offer_ind, _) = pools
        .iter()
        .find_position(|asset| asset.info == offer_asset.info)
        .ok_or_else(|| ContractError::InvalidAsset(offer_asset_dec.info.to_string()))?;
    let ask_ind = 1 - offer_ind;
    let ask_asset_prec = precisions.get_precision(&pools[ask_ind].info)?;

    before_swap_check(&pools, offer_asset_dec.amount)?;

    let xs = pools.iter().map(|asset| asset.amount).collect_vec();

   
    let mut maker_fee_share = Decimal256::zero();
    
    // If this pool is configured to share fees
    let mut share_fee_share = Decimal256::zero();
   
    let swap_result = compute_swap(
        &xs,
        offer_asset_dec.amount,
        ask_ind,
        &config,
        &env,
        maker_fee_share,
        share_fee_share,
    )?;

    Ok(SimulationResponse {
        return_amount: swap_result.dy.to_uint(ask_asset_prec)?,
        spread_amount: swap_result.spread_fee.to_uint(ask_asset_prec)?,
        commission_amount: swap_result.total_fee.to_uint(ask_asset_prec)?,
    })
}
