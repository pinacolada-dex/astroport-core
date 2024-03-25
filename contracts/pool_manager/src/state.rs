use cosmwasm_std::Uint128;
use cw_storage_plus::{Item,Map, SnapshotMap};
use itertools::Itertools;
use astroport::asset::{AssetInfo,Asset};



use astroport_pcl_common::state::Config;
use cosmwasm_std::{DepsMut};
/// Stores pool parameters and state.


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
    
    ASSET TYPES
    #[derive(Hash, Eq)]
    pub enum AssetInfo {
        /// Non-native Token
        Token { contract_addr: Addr },
        /// Native token
        NativeToken { denom: String },
    }
   
    #[cw_serde]
    pub struct Asset {
        /// Information about an asset stored in a [`AssetInfo`] struct
        pub info: AssetInfo,
        /// A token amount
        pub amount: Uint128,
    }

    /// This struct describes a Terra asset as decimal.
    #[cw_serde]
    pub struct DecimalAsset {
        pub info: AssetInfo,
        pub amount: Decimal256,
    }
    que
 */
pub const QUEUED_MINT: Item<String> = Item::new("pool_key");
pub const POOLS:Map<String,Config> = Map::new("pools");
pub const PAIR_BALANCES:Map<String,Vec<Asset>> = Map::new("pair_balances");
/// Stores asset balances to query them later at any block height
pub const BALANCES: SnapshotMap<&AssetInfo, Uint128> = SnapshotMap::new(
    "balances",
    "balances_check",
    "balances_change",
    cw_storage_plus::Strategy::EveryBlock,
);

pub fn increment_pair_balances(deps:DepsMut,key:String,amounts:Vec<Uint128>){
    let mut curr=PAIR_BALANCES.load(deps.storage,key.clone()).unwrap();
    for (i,v) in amounts.into_iter().enumerate(){
        curr[i].amount-=v;
    }
    PAIR_BALANCES.save(deps.storage,key,&curr);
}

pub fn decrease_pair_balances(deps:DepsMut,key:String,amounts:Vec<Uint128>){
    let mut curr=PAIR_BALANCES.load(deps.storage,key.clone()).unwrap();
    for (i,v) in amounts.into_iter().enumerate(){
        curr[i].amount-=v;
    }
    PAIR_BALANCES.save(deps.storage,key,&curr);
}

pub fn pair_key(asset_infos: &[AssetInfo]) -> Vec<u8> {
    asset_infos
        .iter()
        .map(AssetInfo::as_bytes)
        .sorted()
        .flatten()
        .copied()
        .collect()
}

