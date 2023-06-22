use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;

use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{
    from_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Empty, OwnedDeps, Reply,
    Response, SubMsgResponse, SubMsgResult, Uint128,
};
use osmosis_std::types::osmosis::poolmanager::v1beta1::{
    MsgSwapExactAmountIn, MsgSwapExactAmountInResponse, SwapAmountInRoute,
};

use crate::contract::ExecMsg;
use crate::contract::{AffiliateSwap, ContractExecMsg, SwapResponse};
use crate::{execute, reply};

fn setup_unit(fee: Option<Decimal>) -> OwnedDeps<MockStorage, MockApi, MockQuerier, Empty> {
    let affiliate_swap = AffiliateSwap::new();
    let mut deps = mock_dependencies();
    // instantiate contract
    affiliate_swap
        .instantiate(
            (deps.as_mut(), mock_env(), mock_info("instantiator", &[])),
            fee,
        )
        .unwrap();

    deps
}

const SENDER: &str = "sender";
const COLLECTOR: &str = "collector";

fn simple_execute(deps: DepsMut, amount: u128, fee: Option<Decimal>) -> Response {
    execute(
        deps,
        mock_env(),
        mock_info(SENDER, &[Coin::new(amount, "uosmo")]),
        ContractExecMsg::AffiliateSwap(ExecMsg::Swap {
            routes: vec![SwapAmountInRoute {
                pool_id: 1,
                token_out_denom: "uion".to_string(),
            }],
            token_out_min_amount: Coin::new(1, "uion"),
            fee_percentage: fee,
            fee_collector: COLLECTOR.to_string(),
        }),
    )
    .unwrap()
}

fn is_valid_swap_msg(msg: &CosmosMsg, token_in: Coin) -> bool {
    match msg {
        CosmosMsg::Stargate { type_url, value } => {
            let basic = type_url == "/osmosis.poolmanager.v1beta1.MsgSwapExactAmountIn"
                && !value.as_slice().is_empty();
            let swap_value = {
                let swap_msg: MsgSwapExactAmountIn = value.clone().try_into().expect("bad msg");
                //println!("{:?}", swap_msg.token_in);
                swap_msg.token_in.unwrap() == token_in.into()
            };
            basic && swap_value
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
    let affiliate_swap = AffiliateSwap::new();
    let mut deps = setup_unit(Some(Decimal::from_str("5").unwrap()));

    // No fee set, no fee taken
    let res = simple_execute(deps.as_mut(), 100, None);
    assert_eq!(res.messages.len(), 1);
    assert!(is_valid_swap_msg(
        &res.messages[0].msg,
        Coin::new(100, "uosmo")
    ));

    // delete the active swap. This would normally be handled by the reply
    affiliate_swap.active_swap.remove(&mut deps.storage);

    // Fee 1%, swap 99%
    let res = simple_execute(deps.as_mut(), 100, Some(Decimal::from_str("1").unwrap()));
    assert_eq!(res.messages.len(), 2);
    assert!(is_valid_bank_send_msg(
        &res.messages[0].msg,
        COLLECTOR,
        1u128.into(),
        "uosmo"
    ));
    assert!(is_valid_swap_msg(
        &res.messages[1].msg,
        Coin::new(99, "uosmo")
    ),);

    // delete the active swap. This would normally be handled by the reply
    affiliate_swap.active_swap.remove(&mut deps.storage);

    // Fee 10%, defaults to max: 5%
    let res = simple_execute(deps.as_mut(), 100, Some(Decimal::from_str("10").unwrap()));
    assert_eq!(res.messages.len(), 2);
    assert!(is_valid_bank_send_msg(
        &res.messages[0].msg,
        COLLECTOR,
        5u128.into(),
        "uosmo"
    ));
    assert!(is_valid_swap_msg(
        &res.messages[1].msg,
        Coin::new(95, "uosmo")
    ));

    // delete the active swap. This would normally be handled by the reply
    affiliate_swap.active_swap.remove(&mut deps.storage);

    // Non-int fee
    let res = simple_execute(deps.as_mut(), 1000, Some(Decimal::from_str("1.7").unwrap()));
    assert_eq!(res.messages.len(), 2);
    assert!(is_valid_bank_send_msg(
        &res.messages[0].msg,
        COLLECTOR,
        17u128.into(),
        "uosmo"
    ));
    assert!(is_valid_swap_msg(
        &res.messages[1].msg,
        Coin::new(983, "uosmo")
    ));

    // delete the active swap. This would normally be handled by the reply
    affiliate_swap.active_swap.remove(&mut deps.storage);

    // Edge cases

    // low amounts send amount: no fee taken
    let res = simple_execute(deps.as_mut(), 1, Some(Decimal::from_str("1").unwrap()));
    assert_eq!(res.messages.len(), 1);
    assert!(is_valid_swap_msg(
        &res.messages[0].msg,
        Coin::new(1, "uosmo")
    ));
    // delete the active swap. This would normally be handled by the reply
    affiliate_swap.active_swap.remove(&mut deps.storage);

    // Fee rounds to less than one: no fee taken
    let res = simple_execute(deps.as_mut(), 9, Some(Decimal::from_str("10").unwrap()));
    assert_eq!(res.messages.len(), 1);
    assert!(is_valid_swap_msg(
        &res.messages[0].msg,
        Coin::new(9, "uosmo")
    ));
    // delete the active swap. This would normally be handled by the reply
    affiliate_swap.active_swap.remove(&mut deps.storage);

    // Fee rounds to at least one: fee taken
    let res = simple_execute(deps.as_mut(), 20, Some(Decimal::from_str("5").unwrap()));
    assert_eq!(res.messages.len(), 2);
    assert!(is_valid_bank_send_msg(
        &res.messages[0].msg,
        COLLECTOR,
        1u128.into(),
        "uosmo"
    ));
    assert!(is_valid_swap_msg(
        &res.messages[1].msg,
        Coin::new(19, "uosmo")
    ));
    // delete the active swap. This would normally be handled by the reply
    affiliate_swap.active_swap.remove(&mut deps.storage);

    // Max uint amount
    let res = simple_execute(deps.as_mut(), Uint128::MAX.into(), Some(Decimal::from_str("5").unwrap()));
    assert_eq!(res.messages.len(), 2);
    let fee = Uint128::MAX*(Decimal::from_str("5").unwrap().checked_div(Decimal::from_str("100").unwrap()).unwrap());
    assert!(is_valid_bank_send_msg(
        &res.messages[0].msg,
        COLLECTOR,
        fee,
        "uosmo"
    ));
    assert!(is_valid_swap_msg(
        &res.messages[1].msg,
        Coin::new((Uint128::MAX-fee).into(), "uosmo")
    ));
    // delete the active swap. This would normally be handled by the reply
    affiliate_swap.active_swap.remove(&mut deps.storage);
}

fn simple_reply(deps: DepsMut, amount: impl Display) -> Response {
    reply(
        deps,
        mock_env(),
        Reply {
            id: 1,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(
                    MsgSwapExactAmountInResponse {
                        token_out_amount: amount.to_string(),
                    }
                    .into(),
                ),
            }),
        },
    )
    .unwrap()
}

#[test]
fn test_reply() {
    let mut deps = setup_unit(Some(Decimal::from_str("5").unwrap()));

    simple_execute(deps.as_mut(), 100, Some(Decimal::from_str("1").unwrap()));
    let res = simple_reply(deps.as_mut(), 98);

    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: SENDER.to_string(),
            amount: vec![Coin::new(98, "uion")],
        })
    );

    // The active swap has been deleted
    let affiliate_swap = AffiliateSwap::new();
    affiliate_swap.active_swap.load(&deps.storage).unwrap_err();

    // get the event
    let event = res
        .events
        .iter()
        .find(|e| e.ty == "affiliate_swap")
        .unwrap();

    let event_attributes = event
        .attributes
        .iter()
        .map(|a| (a.key.clone(), a.value.clone()))
        .collect::<HashMap<_, _>>();
    assert_eq!(event_attributes["sender"], SENDER);
    assert_eq!(event_attributes["swap_token_in"], "99uosmo");
    assert_eq!(event_attributes["token_out"], "98uion");
    assert_eq!(event_attributes["fee"], "1uosmo");

    // check data
    let response: SwapResponse = from_binary(&res.data.unwrap()).unwrap();
    assert_eq!(
        response,
        SwapResponse {
            original_sender: SENDER.to_string(),
            fee: 1_u128.into(),
            fee_collector: Addr::unchecked(COLLECTOR),
            swap_in_denom: "uosmo".to_string(),
            swap_in_amount: 99_u128.into(),
            token_out_denom: "uion".to_string(),
            token_out_amount: 98_u128.into(),
        }
    );
}

#[test]
fn test_bad_reply() {
    let mut deps = setup_unit(Some(Decimal::from_str("5").unwrap()));
    simple_execute(deps.as_mut(), 100, Some(Decimal::from_str("1").unwrap()));
    reply(
        deps.as_mut(),
        mock_env(),
        Reply {
            id: 1,
            result: SubMsgResult::Err("Any error should do here".to_string()),
        },
    )
    .unwrap_err();
}
