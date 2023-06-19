use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    coins, to_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, Event,
    MessageInfo, Reply, Response, SubMsg, SubMsgResponse, SubMsgResult, Uint128,
};
use cw_storage_plus::Item;
use osmosis_std::types::osmosis::{
    gamm::v1beta1::MsgSwapExactAmountInResponse,
    poolmanager::v1beta1::{MsgSwapExactAmountIn, SwapAmountInRoute},
};
use std::str::FromStr;
use sylvia::contract;

use crate::error::ContractError;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:affiliate_swap";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const DEFAULT_MAX_FEE: &str = "1.5";
pub const TRUE_MAX_FEE: &str = "10";

// Temporary storage of active swap
#[cw_serde]
pub struct ActiveSwap {
    pub original_sender: Addr,
    pub fee: Coin,
    pub fee_collector: Addr,
    pub swap_msg: MsgSwapExactAmountIn,
}

pub struct AffiliateSwap<'a> {
    pub(crate) max_fee_percentage: Item<'a, Decimal>,
    pub(crate) active_swap: Item<'a, ActiveSwap>,
}

#[contract(error=ContractError)]
impl<'a> AffiliateSwap<'a> {
    /// Create an AffiliateSwap instance.
    pub const fn new() -> Self {
        Self {
            max_fee_percentage: Item::new("max_fee"),
            active_swap: Item::new("active_swap"),
        }
    }

    /// Instantiate the contract.
    #[msg(instantiate)]
    pub fn instantiate(
        &self,
        ctx: (DepsMut, Env, MessageInfo),
        max_fee_percentage: Option<Decimal>,
    ) -> Result<Response, ContractError> {
        let (deps, _env, _info) = ctx;

        // store contract version for migration info
        cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        let max_fee = max_fee_percentage.unwrap_or(Decimal::from_str(DEFAULT_MAX_FEE)?);
        if max_fee < Decimal::zero() || max_fee > Decimal::from_str(TRUE_MAX_FEE)? {
            return Err(ContractError::InvalidMaxFeePercentage {
                true_max_fee: TRUE_MAX_FEE.to_string(),
            });
        }

        // set the max fee
        self.max_fee_percentage.save(deps.storage, &max_fee)?;

        Ok(Response::new()
            .add_attribute("method", "instantiate")
            .add_attribute("contract_name", CONTRACT_NAME)
            .add_attribute("contract_version", CONTRACT_VERSION))
    }

    /// Executes a swap and charges the affiliate fee.
    /// The affiliate fee is deducted from the swap amount and sent to the affiliate address.
    #[msg(exec)]
    pub fn swap(
        &self,
        ctx: (DepsMut, Env, MessageInfo),
        routes: Vec<SwapAmountInRoute>,
        token_out_min_amount: Coin,
        fee_percentage: Option<Decimal>,
        fee_collector: String,
    ) -> Result<Response, ContractError> {
        let (deps, env, info) = ctx;

        // Safety check: No active swap
        if self.active_swap.may_load(deps.storage)?.is_some() {
            // This should never happen as long as the contract isn't called concurrently
            return Err(ContractError::ActiveSwapExists {});
        }

        // ensure funds not empty
        let coin = cw_utils::one_coin(&info)?;

        // validate fee collector address
        let fee_collector = deps.api.addr_validate(&fee_collector)?;

        let max_fee_percentage = self.max_fee_percentage.load(deps.storage)?;

        // Ensure the provided fee percentage is >=0
        // If it is None, default to zero
        let fee_percentage = fee_percentage
            .unwrap_or(Decimal::zero())
            .max(Decimal::zero());

        // Ensure the provided fee percentage is <= max_fee_percentage
        // If it is higher, default to max_fee_percentage
        let fee_percentage = std::cmp::min(max_fee_percentage, fee_percentage);

        // calculate the fee to deduct
        let fee = coin.amount * fee_percentage.checked_div(Decimal::from_str("100")?)?;

        // Add the messages but skip the fee transfer if it is zero
        let mut msgs = vec![];

        if !fee.is_zero() {
            let send_msg: CosmosMsg = BankMsg::Send {
                to_address: fee_collector.to_string(),
                amount: vec![Coin {
                    denom: coin.denom.clone(),
                    amount: fee.into(),
                }],
            }
            .into();
            msgs.push(SubMsg::new(send_msg));
        }

        let swap_msg = MsgSwapExactAmountIn {
            sender: env.contract.address.to_string(),
            routes,
            token_in: Some(
                Coin {
                    denom: coin.denom.clone(),
                    amount: coin.amount - fee,
                }
                .into(),
            ),
            token_out_min_amount: token_out_min_amount.amount.to_string(),
        };
        msgs.push(SubMsg::reply_always(swap_msg.clone(), 1));

        self.active_swap.save(
            deps.storage,
            &ActiveSwap {
                original_sender: info.sender,
                fee_collector,
                fee: Coin {
                    denom: coin.denom,
                    amount: fee,
                },
                swap_msg,
            },
        )?;

        Ok(Response::new()
            .add_submessages(msgs)
            .add_attribute("method", "swap"))
    }

    #[msg(query)]
    pub fn get_max_fee_percentage(
        &self,
        ctx: (Deps, Env),
    ) -> Result<MaxFeePercentageResponse, ContractError> {
        let (deps, _env) = ctx;
        let max_fee_percentage = self.max_fee_percentage.load(deps.storage)?;
        Ok(MaxFeePercentageResponse { max_fee_percentage })
    }

    pub fn reply(&self, ctx: (DepsMut, Env), msg: Reply) -> Result<Response, ContractError> {
        let (deps, _env) = ctx;
        let active_swap = self.active_swap.load(deps.storage)?;
        self.active_swap.remove(deps.storage);

        // Success
        deps.api.debug(&format!("Reply: {:?}", msg));
        if let SubMsgResult::Ok(SubMsgResponse { data: Some(b), .. }) = msg.result {
            let res: MsgSwapExactAmountInResponse = b.try_into()?;

            let amount = Uint128::from_str(&res.token_out_amount)?;
            let token_out_denom = &active_swap
                .swap_msg
                .routes
                .last()
                .ok_or(ContractError::Unexpected {})?
                .token_out_denom;

            let bank_msg = BankMsg::Send {
                to_address: active_swap.original_sender.clone().into_string(),
                amount: coins(amount.u128(), token_out_denom.clone()),
            };

            let token_in: Coin = coinvert(
                active_swap
                    .swap_msg
                    .token_in
                    .ok_or(ContractError::Unexpected {})?,
            )?;

            let response = SwapResponse {
                original_sender: active_swap.original_sender.into_string(),
                fee: active_swap.fee.amount,
                fee_collector: active_swap.fee_collector,
                swap_in_amount: token_in.amount,
                swap_in_denom: token_in.clone().denom,
                token_out_denom: token_out_denom.to_string(),
                token_out_amount: amount,
            };

            return Ok(Response::new()
                .add_message(bank_msg)
                .set_data(to_binary(&response)?)
                .add_event(
                    Event::new("affiliate_swap")
                        .add_attribute("sender", response.original_sender)
                        .add_attribute("swap_token_in", token_in.to_string())
                        .add_attribute("fee", active_swap.fee.to_string())
                        .add_attribute(
                            "token_out",
                            Coin {
                                denom: token_out_denom.to_string(),
                                amount: amount.into(),
                            }
                            .to_string(),
                        ),
                ));
        }

        // Failure
        Err(ContractError::FailedSwap {
            reason: msg.result.unwrap_err(),
        })
    }
}

#[cw_serde]
pub struct MaxFeePercentageResponse {
    pub max_fee_percentage: Decimal,
}

// Response for Swap
#[cw_serde]
pub struct SwapResponse {
    pub original_sender: String,
    pub fee: Uint128,
    pub fee_collector: Addr,
    pub swap_in_denom: String,
    pub swap_in_amount: Uint128,
    pub token_out_denom: String,
    pub token_out_amount: Uint128,
}

// Convert a cosmos proto Coin to a cosmwasm Coin
fn coinvert(
    coin: osmosis_std::types::cosmos::base::v1beta1::Coin,
) -> Result<Coin, cosmwasm_std::StdError> {
    Ok(Coin {
        denom: coin.denom,
        amount: Uint128::from_str(&coin.amount)?,
    })
}
