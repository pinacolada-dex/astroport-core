use cosmwasm_std::Uint128;
use cw_storage_plus::{Item,Map, SnapshotMap};

use astroport::asset::AssetInfo;
use astroport::common::OwnershipProposal;
use astroport::observation::Observation;
use astroport_circular_buffer::CircularBuffer;
use astroport_pcl_common::state::Config;

/// Stores pool parameters and state.
pub const POOLS:Map<String,Config> = Map:new("pools");

/// Stores the latest contract ownership transfer proposal
/**
 * EXAMPLE CONFIG
 * let pool_state = PoolState {
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
 */
pub struct Pair_Balance{
    tokenA:Uint128,
    tokenB:Uint128
}
pub const PAIR_BALANCE:Map<String,Pair_Balance> = Map:new("pair_balances");
/// Stores asset balances to query them later at any block height
pub const BALANCES: SnapshotMap<&AssetInfo, Uint128> = SnapshotMap::new(
    "balances",
    "balances_check",
    "balances_change",
    cw_storage_plus::Strategy::EveryBlock,
);


impl Config{
    //Create key by ordering the pair token Addresses Alphabetically then concatenating
    fn create_key(&self)->String{
        let key=format!("{}{}",self.pair_info.asset_infos[0],self.pair_info.asset_infos[1]);
        String::from(key);
    }
}

