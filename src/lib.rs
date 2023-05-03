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
#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize, Debug)]
pub struct Product {
    product_id: String,
    name: String,
    ipfs: String,
    price: u64,
    total_supply: u64,
    timeout: u8,
    is_discount: bool,
    discount_percent: u8,
    token_amount: u32,
    is_reward: bool,
    reward_amount: u32,
    time_created: u64,
    custom: bool,
    user: Option<String>,
}

#[near_bindgen]
#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub struct Metadata {
    success: bool,
    product_id: String,
    buyer_account_id: AccountId,
    quantity: u64,
    amount: u64,
    store_contract_id: AccountId,
}

#[near_bindgen]
impl Metadata {
    fn new(success: bool, product_id: String, buyer_account_id: AccountId, quantity: u64, amount: u64, store_contract_id: AccountId,) -> Metadata {
        Metadata {
            success,
            product_id,
            buyer_account_id,
            quantity,
            amount,
            store_contract_id,
        }
    }
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

    fn assert_only_owner(&self) {
        assert_one_yocto();
        assert_eq!(
            env::signer_account_id(),
            self.owner_id,
            "Only contract owner can call this method"
        );
    }

    fn assert_only_pipar(&self) {
        assert_one_yocto();
        assert_eq!(
            env::predecessor_account_id(),
            self.contract_id,
            "Only pipar escrow contract can call this method"
        );
    }

    pub fn get_store_products(&self) -> Vector<Product> {
        self.products.into_iter().collect();
    }

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
        price: u64,
        total_supply: u64,
        timeout: u8,
        is_discount: bool,
        discount_percent: u8,
        token_amount: u32,
        is_reward: bool,
        reward_amount: u32,
        custom: bool,
        user: Option<String>,
    ) {
        self.assert_only_owner();
        self.products.push(&Product {
            product_id: Uuid::new_v4().to_string(),
            name: name.parse().unwrap(),
            ipfs: ipfs.parse().unwrap(),
            price,
            total_supply,
            timeout,
            is_discount,
            discount_percent,
            token_amount,
            is_reward,
            reward_amount,
            time_created: env::block_timestamp(),
            custom,
            user
        });
    }

    pub fn store_purchase_product(
        &mut self,
        product_id: String,
        buyer_account_id: AccountId,
        attached_near: u64
    ) {
        self.assert_only_pipar();

        for (index, value) in self.products.iter().enumerate() {
            if value.product_id == product_id {
               let mut obj = self.products.get(index as u64).clone();
                let buyer_id = buyer_account_id.clone();
                assert!(
                    &value.price >= &attached_near,
                    "Attached attachedNear is not enough to buy the product"
                );
                let quantity = &attached_near / &value.price;
                assert!(
                    &value.total_supply >= &quantity,
                    "Seller does not have enough product"
                );
                let new_supply = &value.total_supply - &quantity;

                self.products.replace(index as u64, &Product {
                    product_id: value.product_id,
                    name: value.name,
                    ipfs: value.ipfs,
                    price: value.price,
                    total_supply: new_supply,
                    timeout: value.timeout,
                    is_discount: value.is_discount,
                    discount_percent: value.discount_percent,
                    token_amount: value.token_amount,
                    is_reward: value.is_reward,
                    reward_amount: value.reward_amount,
                    time_created: value.time_created,
                    custom: value.custom,
                    user: value.user
                });
                let metadata = &Metadata::new(true, obj.product_id, buyer_id, obj.quantity, obj.amount, obj.store_contract_id);
                metadata
            } else {
                env::panic_str("Product not found.");
            }
        }

    }

}


