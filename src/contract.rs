use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    BankMsg, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, Response, Uint128,
};
use cw_storage_plus::Item;
use osmosis_std::types::osmosis::poolmanager::v1beta1::{MsgSwapExactAmountIn, SwapAmountInRoute};
use std::str::FromStr;
use sylvia::contract;

use crate::error::ContractError;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:transmuter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_MAX_FEE: &str = "5";

pub struct AffiliateSwap<'a> {
    pub(crate) max_fee_percentage: Item<'a, Decimal>,
}

#[contract]
impl<'a> AffiliateSwap<'a> {
    /// Create a Transmuter instance.
    pub const fn new() -> Self {
        Self {
            max_fee_percentage: Item::new("max_fee"),
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
        if max_fee < Decimal::zero() || max_fee > Decimal::from_str("50")? {
            return Err(ContractError::InvalidMaxFeePercentage {});
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
        affiliate_address: String,
    ) -> Result<Response, ContractError> {
        let (deps, _env, info) = ctx;

        // ensure funds not empty
        let coin = cw_utils::one_coin(&info)?;

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
            deps.api.addr_validate(&affiliate_address)?;
            let send_msg: CosmosMsg = BankMsg::Send {
                to_address: affiliate_address,
                amount: vec![Coin {
                    denom: coin.denom.clone(),
                    amount: fee.into(),
                }],
            }
            .into();
            msgs.push(send_msg);
        }

        let swap_msg = MsgSwapExactAmountIn {
            sender: info.sender.into_string(),
            routes,
            token_in: Some(
                Coin {
                    denom: coin.denom,
                    amount: coin.amount - fee,
                }
                .into(),
            ),
            token_out_min_amount: token_out_min_amount.amount.to_string(),
        };
        msgs.push(swap_msg.into());

        Ok(Response::new()
            .add_messages(msgs)
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
}

#[cw_serde]
pub struct MaxFeePercentageResponse {
    pub max_fee_percentage: Decimal,
}
