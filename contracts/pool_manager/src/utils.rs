pub(crate) fn query_pools(
    deps:&DepsMut,      
    config: &Config,
    precisions: &Precisions,
) -> Result<Vec<DecimalAsset>, ContractError> {
    let key=format!("{}{}",conifg.pair_info.asset_infos[0],config.pair_info.asset_infos[1]);
    //let pools=PAIR_BALANCES.load(key,deps.storage);
    PAIR_BALANCES.load(key,deps.storage)
    .into_iter()
    .map(|asset| {
        asset
            .to_decimal_asset(precisions.get_precision(&asset.info)?)
            .map_err(Into::into)
    })
    .collect()
    
}
