use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Decimal, Deps, DepsMut, Env, MessageInfo, Response};
use cw_storage_plus::Item;
use sylvia::contract;

use crate::error::ContractError;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:transmuter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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

        let max_fee = max_fee_percentage.unwrap_or(Decimal::percent(5));
        if max_fee < Decimal::percent(0) || max_fee > Decimal::percent(100) {
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
    pub fn swap(&self, ctx: (DepsMut, Env, MessageInfo)) -> Result<Response, ContractError> {
        let (deps, _env, info) = ctx;

        // ensure funds not empty
        ensure!(
            !info.funds.is_empty(),
            ContractError::AtLeastSingleTokenExpected {}
        );

        Ok(Response::new().add_attribute("method", "swap"))
    }

    #[msg(query)]
    pub fn get_max_fee_percentage(
        &self,
        ctx: (Deps, Env),
    ) -> Result<MaxFeePercentageResponse, ContractError> {
        let (deps, _env) = ctx;
        Ok(MaxFeePercentageResponse {})
    }
}

#[cw_serde]
pub struct MaxFeePercentageResponse {}
