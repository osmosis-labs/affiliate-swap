use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Invalid max fee percentage. Must be between 0 and 100")]
    InvalidMaxFeePercentage {},

    #[error("Funds must contain at least one token")]
    AtLeastSingleTokenExpected {},
}
