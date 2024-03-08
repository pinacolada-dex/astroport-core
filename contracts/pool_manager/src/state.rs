use cosmwasm_std::Uint128;
use cw_storage_plus::{Item,Map, SnapshotMap};

use astroport::asset::AssetInfo;
use astroport::common::OwnershipProposal;
use astroport::observation::Observation;
use astroport_circular_buffer::CircularBuffer;
use astroport_pcl_common::state::Config;

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
 */
pub const POOLS:Map<String,Config> = Map:new("pools");
pub const PAIR_BALANCES:Map<String,Vec<Asset>> = Map:new("pair_balances");
/// Stores asset balances to query them later at any block height
pub const BALANCES: SnapshotMap<&AssetInfo, Uint128> = SnapshotMap::new(
    "balances",
    "balances_check",
    "balances_change",
    cw_storage_plus::Strategy::EveryBlock,
);
pub fn generate_key_from_assets(assets:Vec<Asset>)-> String{
    format!("{}{}",assets[0].asset_info,assets[1].asset_info)
}
pub fn increment_pair_balances(deps:DepsMut,key:String,amounts:Vec<Uint128>){
    let curr=PAIR_BALANCES.load(deps.storage,key);
    for i in amounts.len(){
        curr[i].amount=+amounts[i];
    }
    PAIR_BALANCES.save(deps.storeage,key,curr);
}

pub fn decrease_pair_balances(deps:DepsMut,key:String,amounts:Vec<Uint128>){
    let curr=PAIR_BALANCES.load(deps.storage,key);
    for i in amounts.len(){
        curr[i].amount=-amounts[i];
    }
    PAIR_BALANCES.save(deps.storeage,key,curr);
}

impl Config{
    //Create key by ordering the pair token Addresses Alphabetically then concatenating
    fn create_key(&self)->String{
        let key=format!("{}{}",self.pair_info.asset_infos[0],self.pair_info.asset_infos[1]);
        String::from(key);
    }
}

pub struct Precisions(Vec<(String, u8)>);

impl<'a> Precisions {
    /// Stores map of AssetInfo (as String) -> precision
    const PRECISIONS: Map<'a, String, u8> = Map::new("precisions");
    pub fn new(storage: &dyn Storage) -> StdResult<Self> {
        let items = Self::PRECISIONS
            .range(storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;

        Ok(Self(items))
    }

    /// Store all token precisions
    pub fn store_precisions<C: CustomQuery>(
        deps: DepsMut<C>,
        asset_infos: &[AssetInfo],
        factory_addr: &Addr,
    ) -> StdResult<()> {
        for asset_info in asset_infos {
            let precision = asset_info.decimals(&deps.querier, factory_addr)?;
            Self::PRECISIONS.save(deps.storage, asset_info.to_string(), &precision)?;
        }

        Ok(())
    }

    pub fn get_precision(&self, asset_info: &AssetInfo) -> Result<u8, PclError> {
        self.0
            .iter()
            .find_map(|(info, prec)| {
                if info == &asset_info.to_string() {
                    Some(*prec)
                } else {
                    None
                }
            })
            .ok_or_else(|| PclError::InvalidAsset(asset_info.to_string()))
    }
}
