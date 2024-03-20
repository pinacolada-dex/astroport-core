use cosmwasm_std::DepsMut;
use astroport_pcl_common::state::{Config,Precisions};
use astroport::asset::DecimalAsset;
use crate::state::PAIR_BALANCES;
use crate::error::ContractError;
pub(crate) fn query_pools(
    deps:&DepsMut,      
    config: &Config,
    precisions: &Precisions,
) -> Result<Vec<DecimalAsset>, ContractError> {
    let key=format!("{}{}",config.pair_info.asset_infos[0],config.pair_info.asset_infos[1]);
    //let pools=PAIR_BALANCES.load(key,deps.storage);
    PAIR_BALANCES.load(deps.storage,key).unwrap()
    .into_iter()
    .map(|asset| {
        asset
            .to_decimal_asset(precisions.get_precision(&asset.info)?)
            .map_err(Into::into)
    })
    .collect()
    
}
