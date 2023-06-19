use std::str::FromStr;

use cosmwasm_std::{coins, Coin, Decimal};
use osmosis_test_tube::{Account, FeeSetting};

use crate::{
    contract::{ExecMsg, InstantiateMsg, DEFAULT_MAX_FEE, TRUE_MAX_FEE},
    ContractError,
};

use super::{TestEnv, TestEnvBuilder};

fn setup_integration(fee: Option<Decimal>) -> TestEnv {
    TestEnvBuilder::new()
        .with_instantiate_msg(InstantiateMsg {
            max_fee_percentage: fee,
        })
        .build()
}

// Test instantiate with no max fee
#[test]
fn test_instantiate() {
    // If no max_fee is set, the max fee defaults to 1.5%
    let t = setup_integration(None);
    let max_fee = t.query_max_fee();
    assert_eq!(max_fee, Decimal::from_str(DEFAULT_MAX_FEE).unwrap());

    // Test instantiate with a 5% max fee
    let t = setup_integration(Some(Decimal::from_str("5").unwrap()));
    let max_fee = t.query_max_fee();
    assert_eq!(max_fee, Decimal::from_str("5").unwrap());

    // Test instantiate with a 10% max fee
    let t = setup_integration(Some(Decimal::from_str("1").unwrap()));
    let max_fee = t.query_max_fee();
    assert_eq!(max_fee, Decimal::from_str("1").unwrap());

    // Test instantiate with a 0% max fee
    let t = setup_integration(Some(Decimal::from_str("0").unwrap()));
    let max_fee = t.query_max_fee();
    assert_eq!(max_fee, Decimal::from_str("0").unwrap());

    // Test instantiate with a 50% max fee
    let t = setup_integration(Some(Decimal::from_str(TRUE_MAX_FEE).unwrap()));
    let max_fee = t.query_max_fee();
    assert_eq!(max_fee, Decimal::from_str(TRUE_MAX_FEE).unwrap());
}

#[test]
#[should_panic(expected = "Invalid max fee percentage. Must be between 0 and 10")]
fn test_instantiate_with_fee_greater_than_true_max_fee_percent() {
    // convert TRUE_MAX_FEE to a u8
    let max_fee = Decimal::from_str(TRUE_MAX_FEE).unwrap() + Decimal::from_str("1").unwrap();
    TestEnvBuilder::new()
        .with_instantiate_msg(InstantiateMsg {
            max_fee_percentage: Some(Decimal::from_str(format!("{max_fee}").as_str()).unwrap()),
        })
        .build();
}

#[test]
fn test_no_funds_sent() {
    let t = setup_integration(None);
    let err = t
        .wasm()
        .execute(
            &t.contract_addr,
            &ExecMsg::Swap {
                routes: vec![],
                token_out_min_amount: Coin::new(1, "uion"),
                fee_percentage: None,
                fee_collector: String::new(),
            },
            &[],
            &t.accounts[0],
        )
        .unwrap_err();

    assert!(err
        .to_string()
        .contains(&ContractError::Payment(cw_utils::PaymentError::NoFunds {}).to_string()));
}

#[test]
fn test_failed_swap() {
    let t = setup_integration(None);
    let sender = t
        .app
        .init_account(&coins(1_000_000_000_000, "uosmo"))
        .unwrap();
    let sender = sender.with_fee_setting(FeeSetting::Custom {
        amount: Coin::new(1_000_000, "uosmo"),
        gas_limit: 100_000_000,
    });

    let err = t
        .wasm()
        .execute(
            &t.contract_addr,
            &ExecMsg::Swap {
                routes: vec![],
                token_out_min_amount: Coin::new(1, "non-existent"),
                fee_percentage: None,
                fee_collector: t.accounts[1].address(),
            },
            &[Coin::new(1, "uosmo")],
            &sender,
        )
        .unwrap_err();
    // TODO: Figure out how to give better error messages to users
    println!("{}", err);
}
