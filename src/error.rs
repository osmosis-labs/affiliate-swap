use cosmwasm_std::{CheckedFromRatioError, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Payment(#[from] cw_utils::PaymentError),

    #[error("{0}")]
    Math(#[from] CheckedFromRatioError),

    #[error("{0}")]
    Overflow(#[from] cosmwasm_std::OverflowError),

    #[error("Invalid max fee percentage. Must be between 0 and 50")]
    InvalidMaxFeePercentage {},

    #[error("Funds must contain at least one token")]
    AtLeastSingleTokenExpected {},

    #[error("There is already an active swap stored for this contract. Re-entry not allowed.")]
    ActiveSwapExists {},

    #[error("Swap failed: {reason}")]
    FailedSwap { reason: String },

    #[error(
        "Unexpected error. This should never happen as validation should have prevented this."
    )]
    Unexpected {},
}
