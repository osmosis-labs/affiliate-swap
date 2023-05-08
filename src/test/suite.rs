use std::str::FromStr;

use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{
    BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Empty, OwnedDeps, Response, Uint128,
};
use cw_multi_test::Executor;

use crate::contract::{AffiliateSwap, ContractExecMsg, MaxFeePercentageResponse};
use crate::execute;
use crate::test::TestEnv;
use crate::{
    contract::{ExecMsg, InstantiateMsg, QueryMsg},
    test::TestEnvBuilder,
    ContractError,
};

fn setup_integration(fee: Option<Decimal>) -> TestEnv {
    TestEnvBuilder::new()
        .with_instantiate_msg(InstantiateMsg {
            max_fee_percentage: fee,
        })
        .with_account(
            "provider",
            vec![Coin::new(2_000, "uosmo"), Coin::new(2_000, "uion")],
        )
        .build()
}

// Test instantiate with no max fee
#[test]
fn test_instantiate() {
    // If no max_fee is set, the max fee defaults to 5%
    let t = setup_integration(None);
    let max_fee = t.query_max_fee();
    assert_eq!(max_fee, Decimal::from_str("5").unwrap());

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
    let t = setup_integration(Some(Decimal::from_str("50").unwrap()));
    let max_fee = t.query_max_fee();
    assert_eq!(max_fee, Decimal::from_str("50").unwrap());
}

#[test]
#[should_panic(expected = "Invalid max fee percentage. Must be between 0 and 50")]
fn test_instantiate_with_fee_greater_than_50_percent() {
    TestEnvBuilder::new()
        .with_instantiate_msg(InstantiateMsg {
            max_fee_percentage: Some(Decimal::from_str("51").unwrap()),
        })
        .build();
}

#[test]
fn test_no_funds_sent() {
    let mut t = setup_integration(None);

    let sender = t.accounts["provider"].clone();
    let err = t
        .app
        .execute_contract(
            sender,
            t.contract.clone(),
            &ExecMsg::Swap {
                routes: vec![],
                token_out_min_amount: Coin::new(1, "uion"),
                fee_percentage: None,
                affiliate_address: String::new(),
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast_ref::<ContractError>().unwrap(),
        &ContractError::Payment(cw_utils::PaymentError::NoFunds {})
    );
}

fn setup_unit(fee: Option<Decimal>) -> OwnedDeps<MockStorage, MockApi, MockQuerier, Empty> {
    let transmuter = AffiliateSwap::new();
    let mut deps = mock_dependencies();
    // instantiate contract
    transmuter
        .instantiate(
            (deps.as_mut(), mock_env(), mock_info("instantiator", &[])),
            fee,
        )
        .unwrap();

    deps
}

const SENDER: &str = "sender";

fn simple_execute(deps: DepsMut, amount: u128, fee: Option<Decimal>) -> Response {
    execute(
        deps,
        mock_env(),
        mock_info(SENDER, &vec![Coin::new(amount, "uosmo")]),
        ContractExecMsg::AffiliateSwap(ExecMsg::Swap {
            routes: vec![],
            token_out_min_amount: Coin::new(1, "uion"),
            fee_percentage: fee,
            affiliate_address: SENDER.to_string(),
        }),
    )
    .unwrap()
}

fn is_valid_swap_msg(msg: &CosmosMsg) -> bool {
    match msg {
        CosmosMsg::Stargate { type_url, value } => {
            type_url == "/osmosis.poolmanager.v1beta1.MsgSwapExactAmountIn"
                && value.as_slice().len() > 0
        }
        _ => false,
    }
}

fn is_valid_bank_send_msg(msg: &CosmosMsg, receiver: &str, amount: Uint128, denom: &str) -> bool {
    match msg {
        CosmosMsg::Bank(BankMsg::Send {
            to_address,
            amount: coins,
        }) => {
            to_address == receiver
                && coins.len() == 1
                && coins[0].amount == amount
                && coins[0].denom == denom
        }
        _ => false,
    }
}

#[test]
fn test_fee_calculation() {
    let mut deps = setup_unit(None);

    // No fee set, no fee taken
    let res = simple_execute(deps.as_mut(), 100, None);
    assert_eq!(res.messages.len(), 1);
    assert!(is_valid_swap_msg(&res.messages[0].msg));

    // Fee 1%, swap 99%
    let res = simple_execute(deps.as_mut(), 100, Some(Decimal::from_str("1").unwrap()));
    assert_eq!(res.messages.len(), 2);
    assert!(is_valid_bank_send_msg(
        &res.messages[0].msg,
        SENDER,
        1u128.into(),
        "uosmo"
    ));
    assert!(is_valid_swap_msg(&res.messages[1].msg));

    // Fee 10%, defaults to max: 5%
    let res = simple_execute(deps.as_mut(), 100, Some(Decimal::from_str("10").unwrap()));
    assert_eq!(res.messages.len(), 2);
    assert!(is_valid_bank_send_msg(
        &res.messages[0].msg,
        SENDER,
        5u128.into(),
        "uosmo"
    ));
    assert!(is_valid_swap_msg(&res.messages[1].msg));

    // Non-int fee
    let res = simple_execute(deps.as_mut(), 1000, Some(Decimal::from_str("1.7").unwrap()));
    assert_eq!(res.messages.len(), 2);
    assert!(is_valid_bank_send_msg(
        &res.messages[0].msg,
        SENDER,
        17u128.into(),
        "uosmo"
    ));
    assert!(is_valid_swap_msg(&res.messages[1].msg));

    println!("{:?}", res);
}
