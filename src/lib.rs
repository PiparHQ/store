// use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
extern crate uuid;
use near_sdk::{self, assert_one_yocto, collections::{LookupSet, LookupMap, Vector}, borsh::{self, BorshDeserialize, BorshSerialize}, PublicKey};
use near_sdk::{env, log, near_bindgen, AccountId, Gas, Promise, PromiseError, PanicOnDefault, json_types::U128, is_promise_success,};
use serde::{Serialize, Deserialize};
use serde_json;
use uuid::Uuid;

// Constants
pub const ONE_NEAR: u128 = 1_000_000_000_000_000_000_000_000;
pub const TOKEN_BALANCE: u128 = 5_000_000_000_000_000_000_000_000;

pub const fn tgas(n: u64) -> Gas {
    Gas(n * 10u64.pow(12))
}
pub const CREATE_ACCOUNT: Gas = tgas(65 + 5);

#[near_bindgen]
#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub struct Product {
    product_id: String,
    name: String,
    ipfs: String,
    total_supply: u64,
    timeout: String,
    is_discount: bool,
    discount_percent: u8,
    token_amount: u32,
    is_reward: bool,
    reward_amount: u32,
    time_created: String,
    custom: bool,
    user: Option<String>,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct PiparStoreFactory {
    pub products: Vector<Product>,
    pub owner_id: AccountId,
    pub contract_id: AccountId,
    pub token_cost: u128
}

impl Default for PiparStoreFactory {
    fn default() -> Self {
        env::panic_str("Not initialized yet.");
    }
}

#[near_bindgen]
impl PiparStoreFactory {
    pub fn get_token_cost(&self) -> U128 {
        self.token_cost.into()
    }

    /// Initialization
    #[init(ignore_state)]
    pub fn new(&self, owner_id: AccountId, contract_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            products: Vector::new(b"vec-uid-1".to_vec()),
            owner_id,
            contract_id,
            token_cost: TOKEN_BALANCE
        }
    }

    #[private]
    #[init(ignore_state)]
    pub fn migrate() -> Self {
        let old = env::state_read().expect("migrating state");
        Self { ..old }
    }

    #[payable]
    pub fn create_token(new_account_id: String, new_public_key: PublicKey, keypom_args: KeypomArgs) -> Promise {
        let prefix = &new_account_id[0..new_account_id.len()-8];
        let public_key: PublicKey = new_public_key;
        let current_account = env::current_account_id().to_string();
        let subaccount: AccountId = format!("{prefix}.{current_account}").parse().unwrap();
        let init_args = serde_json::to_vec(&FtData {
            owner_id: subaccount.clone(),
            total_supply: "1000000000000000".to_string()
        })
            .unwrap();

        Promise::new(subaccount.clone())
            .create_account()
            .add_full_access_key(public_key)
            .transfer(TOKEN_BALANCE)
            .deploy_contract(include_bytes!("../wasm/ft.wasm").to_vec())
    }

    pub fn add_product(
        &mut self,
        name: String,
        ipfs: String,
        total_supply: u64,
        timeout: String,
        is_discount: bool,
        discount_percent: u8,
        token_amount: u32,
        is_reward: bool,
        reward_amount: u32,
        custom: bool,
        user: Option<String>,
    ) {
        // let mut value = {
        //     product_id: Uuid::new_v4().to_string(),
        //     &name,
        //     &ipfs,
        //     &total_supply: u64,
        //     &timeout: String,
        //     is_discount: bool,
        //     discount_percent: u8,
        //     token_amount: u32,
        //     is_reward: bool,
        //     reward_amount: u32,
        //     time_created: String,
        //     custom: bool,
        //     user: Option<String>,
        // };
    }

}


