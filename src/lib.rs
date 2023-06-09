pub mod contract;

mod error;
pub use crate::error::ContractError;

#[cfg(not(feature = "library"))]
mod entry_points {
    use crate::contract::{AffiliateSwap, ContractExecMsg, ContractQueryMsg, InstantiateMsg};
    use crate::error::ContractError;
    use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response};

    const CONTRACT: AffiliateSwap = AffiliateSwap::new();

    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        msg.dispatch(&CONTRACT, (deps, env, info))
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ContractExecMsg,
    ) -> Result<Response, ContractError> {
        msg.dispatch(&CONTRACT, (deps, env, info))
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: ContractQueryMsg) -> Result<Binary, ContractError> {
        msg.dispatch(&CONTRACT, (deps, env))
    }

    #[entry_point]
    pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
        CONTRACT.reply((deps, env), msg)
    }
}

#[cfg(not(feature = "library"))]
pub use crate::entry_points::*;

#[cfg(test)]
mod test;
