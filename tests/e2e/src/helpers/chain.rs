use cosm_orc::orchestrator::{CosmosgRPC, Key, SigningKey};
use cosm_orc::{config::cfg::Config, orchestrator::cosm_orc::CosmOrc};
use once_cell::sync::OnceCell;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::Path;
use std::time::Duration;
use test_context::TestContext;

use crate::helpers::instantiate::{instantiate_fair_burn, instantiate_marketplace};

static CONFIG: OnceCell<Cfg> = OnceCell::new();

#[derive(Clone, Debug)]
pub struct Cfg {
    pub orc_cfg: Config,
    pub users: Vec<SigningAccount>,
    pub gas_report_dir: String,
}

#[derive(Clone, Debug)]
pub struct SigningAccount {
    pub account: Account,
    pub key: SigningKey,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub name: String,
    pub address: String,
    pub mnemonic: String,
}

// NOTE: we have to run the integration tests in one thread right now.
// We get `account sequence mismatch` CosmosSDK errors when we run in parallel.
// If we want to parallelize them we could either serialize the `account.sequence` per key,
// or use a different key per test.

#[derive(Clone, Debug)]
pub struct Chain {
    pub cfg: Cfg,
    pub orc: CosmOrc<CosmosgRPC>,
}

impl TestContext for Chain {
    fn setup() -> Self {
        let cfg = CONFIG.get_or_init(global_setup).clone();
        let orc = CosmOrc::new(cfg.orc_cfg.clone(), true).unwrap();
        Self { cfg, orc }
    }

    fn teardown(self) {
        let cfg = CONFIG.get().unwrap();
        save_gas_report(&self.orc, &cfg.gas_report_dir);
    }
}

// global_setup() runs once before all of the tests:
// - loads cosm orc / test account config files
// - stores contracts on chain for all tests to reuse
fn global_setup() -> Cfg {
    env_logger::init();

    let config = env::var("CONFIG").expect("missing yaml CONFIG env var");
    let gas_report_dir = env::var("GAS_OUT_DIR").unwrap_or_else(|_| "gas_reports".to_string());

    let mut cfg = Config::from_yaml(&config).unwrap();
    let mut orc = CosmOrc::new(cfg.clone(), true).unwrap();
    let accounts = test_accounts();

    // Poll for first block to make sure the node is up:
    orc.poll_for_n_blocks(1, Duration::from_millis(20_000), true)
        .unwrap();

    let skip_storage = env::var("SKIP_CONTRACT_STORE").unwrap_or_else(|_| "false".to_string());
    if !skip_storage.parse::<bool>().unwrap() {
        orc.store_contracts("../../artifacts", &accounts[0].key, None)
            .unwrap();

        save_gas_report(&orc, &gas_report_dir);
        prep_base_contracts(&mut orc, &accounts[0]);

        // persist stored code_ids in CONFIG, so we can reuse for all tests
        cfg.contract_deploy_info = orc.contract_map.deploy_info().clone();

        // print contract map
        println!("{:#?}", cfg.contract_deploy_info);
    }

    Cfg {
        orc_cfg: cfg,
        users: accounts,
        gas_report_dir,
    }
}

fn test_accounts() -> Vec<SigningAccount> {
    let bytes = fs::read("configs/test_accounts.json").unwrap();
    let accounts: Vec<Account> = serde_json::from_slice(&bytes).unwrap();

    accounts
        .into_iter()
        .map(|a| SigningAccount {
            account: a.clone(),
            key: SigningKey {
                name: a.name,
                key: Key::Mnemonic(a.mnemonic),
            },
        })
        .collect()
}

fn save_gas_report(orc: &CosmOrc<CosmosgRPC>, gas_report_dir: &str) {
    let report = orc
        .gas_profiler_report()
        .expect("error fetching profile reports");

    let j: Value = serde_json::to_value(report).unwrap();

    let p = Path::new(gas_report_dir);
    if !p.exists() {
        fs::create_dir(p).unwrap();
    }

    let mut rng = rand::thread_rng();
    let file_name = format!("test-{}.json", rng.gen::<u32>());
    fs::write(p.join(file_name), j.to_string()).unwrap();
}

fn prep_base_contracts(orc: &mut CosmOrc<CosmosgRPC>, creator: &SigningAccount) {
    instantiate_fair_burn(orc, creator).unwrap();
    instantiate_marketplace(orc, creator).unwrap();
}
