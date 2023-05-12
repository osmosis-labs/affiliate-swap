mod integration;
mod unit;

use crate::contract::{InstantiateMsg, QueryMsg};

use cosmwasm_std::{Coin, Decimal};

use crate::contract::MaxFeePercentageResponse;
use osmosis_test_tube::{Module, OsmosisTestApp, SigningAccount, Wasm};

pub struct TestEnv {
    pub app: OsmosisTestApp,
    pub contract_addr: String,
    accounts: Vec<SigningAccount>,
}

impl TestEnv {
    fn query_max_fee(&self) -> Decimal {
        let max_fee = self
            .wasm()
            .query::<QueryMsg, MaxFeePercentageResponse>(
                &self.contract_addr,
                &QueryMsg::GetMaxFeePercentage {},
            )
            .unwrap();
        max_fee.max_fee_percentage
    }

    fn wasm(&self) -> Wasm<'_, OsmosisTestApp> {
        Wasm::new(&self.app)
    }
}

pub struct TestEnvBuilder {
    instantiate_msg: Option<InstantiateMsg>,
}

impl TestEnvBuilder {
    pub fn new() -> Self {
        Self {
            instantiate_msg: None,
        }
    }

    pub fn with_instantiate_msg(mut self, msg: InstantiateMsg) -> Self {
        self.instantiate_msg = Some(msg);
        self
    }

    pub fn build(self) -> TestEnv {
        let app = OsmosisTestApp::new();

        let accounts = app
            .init_accounts(
                &[
                    Coin::new(1_000_000_000_000, "uion"),
                    Coin::new(1_000_000_000_000, "uosmo"),
                ],
                2,
            )
            .unwrap();

        let wasm = Wasm::new(&app);
        let admin = &accounts[0];

        let wasm_byte_code = std::fs::read("./test_artifacts/affiliate_swap.wasm").unwrap();
        let code_id = wasm
            .store_code(&wasm_byte_code, None, &admin)
            .unwrap()
            .data
            .code_id;

        let contract_addr = wasm
            .instantiate(
                code_id,
                &self.instantiate_msg.unwrap_or(InstantiateMsg {
                    max_fee_percentage: None,
                }),
                None,  // contract admin used for migration, not the same as cw1_whitelist admin
                None,  // contract label
                &[],   // funds
                admin, // signer
            )
            .unwrap()
            .data
            .address;

        TestEnv {
            app,
            contract_addr,
            accounts,
        }
    }
}
