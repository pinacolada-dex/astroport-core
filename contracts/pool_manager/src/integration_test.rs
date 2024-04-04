#![cfg(not(tarpaulin_include))]

use std::error::Error;
use std::fmt::Display;
use std::str::FromStr;

use cosmwasm_std::{coins, from_binary, to_binary, Addr, Decimal, Empty, StdError};
use cw20::Cw20ExecuteMsg;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

use astroport::asset::{native_asset_info, token_asset_info};
use astroport::factory::PairType;
use astroport::router::{
    Cw20HookMsg,InstantiateMsg, MigrateMsg,
};
use astroport::pair_concentrated::{
    ConcentratedPoolConfig, ConcentratedPoolParams, ConcentratedPoolUpdateParams, QueryMsg,
};
use crate::error::ContractError;

use crate::factory_helper::{instantiate_token, mint, mint_native, FactoryHelper};

pub fn common_pcl_params() -> ConcentratedPoolParams {
    ConcentratedPoolParams {
        amp: f64_to_dec(40f64),
        gamma: f64_to_dec(0.000145),
        mid_fee: f64_to_dec(0.0026),
        out_fee: f64_to_dec(0.0045),
        fee_gamma: f64_to_dec(0.00023),
        repeg_profit_threshold: f64_to_dec(0.000002),
        min_price_scale_delta: f64_to_dec(0.000146),
        price_scale: Decimal::one(),
        ma_half_time: 600,
        track_asset_balances: None,
        fee_share: None,
    }
}
pub fn f64_to_dec<T>(val: f64) -> T
where
    T: FromStr,
    T::Err: Error,
{
    T::from_str(&val.to_string()).unwrap()
}

pub fn dec_to_f64(val: impl Display) -> f64 {
    f64::from_str(&val.to_string()).unwrap()
}

fn router_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new_with_empty(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        )
        .with_reply_empty(crate::contract::reply),
    )
}

#[test]
fn pool_manager_works() {
    let mut app = App::default();

    let owner = Addr::unchecked("owner");
   

   
    let router_code = app.store_code(router_contract());
    let pool_manager = app
        .instantiate_contract(
            router_code,
            owner.clone(),
            &InstantiateMsg {
                astroport_factory:String::from("Pina_Colada"),
            },
            &[],
            "router",
            None,
        )
        .unwrap();
        let mut helper = FactoryHelper::init(&mut app, &owner,&pool_manager);
        let token_x = instantiate_token(&mut app, helper.cw20_token_code_id, &owner, "TOX", None);
        let token_y = instantiate_token(&mut app, helper.cw20_token_code_id, &owner, "TOY", None);
        let token_z = instantiate_token(&mut app, helper.cw20_token_code_id, &owner, "TOZ", None);
        for (a, b, typ, liq) in [
            (&token_x, &token_y, PairType::Xyk {}, 100_000_000000),
            (&token_y, &token_z, PairType::Stable {}, 1_000_000_000000),
        ] {
            let params=Some(to_binary(&common_pcl_params()).unwrap());
            let pair = helper
                .create_pair(
                    &mut app,
                    &owner,
                    typ,
                    [token_asset_info(a.clone()), token_asset_info(b.clone())],
                    params,
                )
                .unwrap();
            mint(&mut app, &owner, a, liq, &pair).unwrap();
            mint(&mut app, &owner, b, liq, &pair).unwrap();
            
        }
    }
   


