extern crate uuid;
use near_sdk::{self, assert_one_yocto, collections::{Vector}, borsh::{self, BorshDeserialize, BorshSerialize},};
use near_sdk::{ext_contract, env, near_bindgen, AccountId, Gas, Promise, json_types::U128, is_promise_success, Balance};
use serde::{Serialize, Deserialize};
use serde_json;
use uuid::Uuid;
use std::fmt;

// Constants
pub const ONE_NEAR: u128 = 1_000_000_000_000_000_000_000_000;
pub const TOKEN_BALANCE: u128 = 4_000_000_000_000_000_000_000_000;
pub const NO_DEPOSIT: Balance = 0;
pub const ONE_YOCTO: Balance = 1;

pub const fn tgas(n: u64) -> Gas {
    Gas(n * 10u64.pow(12))
}
pub const CREATE_ACCOUNT: Gas = tgas(65 + 5);

#[near_bindgen]
#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize, fmt::Debug)]
pub struct Product {
    product_id: String,
    name: String,
    ipfs: String,
    price: u128,
    total_supply: u128,
    timeout: u8,
    is_discount: bool,
    discount_percent: u8,
    token_amount: u128,
    is_reward: bool,
    reward_amount: u32,
    time_created: u64,
    custom: bool,
    user: Option<String>,
}

#[near_bindgen]
#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub struct FtData {
    owner_id: AccountId,
    total_supply: String,
    name: String,
    symbol: String,
    icon: String,
}

#[near_bindgen]
#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub struct StorageData {
    account_id: AccountId,
    registration_only: bool,
}

#[near_bindgen]
#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub struct TokenData {
    receiver_id: AccountId,
    registration_only: bool,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct PiparStoreFactory {
    pub products: Vector<Product>,
    pub owner_id: AccountId,
    pub contract_id: AccountId,
    pub token: bool,
    pub token_cost: u128
}

impl Default for PiparStoreFactory {
    fn default() -> Self {
        env::panic_str("Not initialized yet.")
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
        )
    }

    fn assert_only_pipar(&self) {
        assert_one_yocto();
        assert_eq!(
            env::predecessor_account_id(),
            self.contract_id,
            "Only pipar escrow contract can call this method"
        )
    }

    fn assert_enough_deposit(&self) {
        assert_one_yocto();
        assert!(
            env::attached_deposit() >= TOKEN_BALANCE,
            "Please attach enough token balance"
        )
    }

    fn assert_token_false(&self) {
        assert_eq!(
            false,
            self.token,
            "Store owner has already deployed a token"
        )
    }

    pub fn get_store_products(&self) {
        for (index, product) in self.products.iter().enumerate() {
            println!("Element at index {:?}: {:?}", index, product)
        }
    }

    pub fn get_token_cost(&self) -> U128 {
        self.token_cost.into()
    }

    pub fn has_token(&self) -> bool {
        self.token.into()
    }

    /// Initialization
    #[init(ignore_state)]
    pub fn new(owner_id: AccountId, contract_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            products: Vector::new(b"vec-uid-1".to_vec()),
            owner_id,
            contract_id,
            token: false,
            token_cost: TOKEN_BALANCE
        }
    }

    #[private]
    #[init(ignore_state)]
    pub fn migrate() -> Self {
        let old = env::state_read().expect("migrating state");
        Self { ..old }
    }

    #[private]
    pub fn deploy_token_callback(
        &mut self,
        token_creator_id: AccountId,
        attached_deposit: U128,
    ) {
        let attached_deposit: u128 = attached_deposit.into();
        if is_promise_success() {
            env::log_str("Successful token deployment")
        } else {
            Promise::new(token_creator_id)
                .transfer(attached_deposit);
            env::log_str("failed token deployment")
        }
    }

    #[payable]
    pub fn deploy_token(&mut self, total_supply: String, name: String, symbol: String, icon: String) -> Promise {
        self.assert_token_false();
        self.assert_enough_deposit();
        let current_account = env::current_account_id().to_string();
        let subaccount: AccountId = format!("ft.{current_account}").parse().unwrap();
        assert!(
            env::is_valid_account_id(subaccount.as_bytes()),
            "Invalid subaccount"
        );
        let init_args = serde_json::to_vec(&FtData {
            owner_id: subaccount.clone(),
            total_supply,
            name,
            symbol,
            icon
        })
            .unwrap();

        Promise::new(subaccount.clone())
            .create_account()
            .add_full_access_key(env::signer_account_pk())
            .transfer(TOKEN_BALANCE)
            .deploy_contract(include_bytes!("../wasm/pipar_fungible_token.wasm").to_vec())
            .function_call("new_default_meta".to_owned(), init_args, NO_DEPOSIT, CREATE_ACCOUNT)
            .then(
                Self::ext(env::current_account_id())
                    .deploy_token_callback(
                        env::predecessor_account_id(),
                        env::attached_deposit().into(),
                    )
            )
    }

    pub fn add_product(
        &mut self,
        name: String,
        ipfs: String,
        price: u128,
        total_supply: u128,
        timeout: u8,
        is_discount: bool,
        discount_percent: u8,
        token_amount: u128,
        is_reward: bool,
        reward_amount: u32,
        custom: bool,
        user: Option<String>,
    ) {
        self.assert_only_owner();
        let id = Uuid::new_v4().to_string();
        self.products.push(&Product {
            product_id: id,
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
        })
    }

    pub fn store_purchase_product(
        &mut self,
        product_id: String,
        buyer_account_id: AccountId,
        attached_near: Balance,
    ) {
        self.assert_only_pipar();

        let product_index = self.products
            .iter()
            .position(|p| p.product_id == product_id)
            .unwrap();

        match self.products.get(product_index as u64) {
            Some(product) => {
                assert!(
                    &product.price >= &attached_near,
                    "Attached attachedNear is not enough to buy the product"
                );
                let quantity = attached_near / &product.price;
                assert!(
                    &product.total_supply >= &quantity,
                    "Seller does not have enough product"
                );
                let new_supply = &product.total_supply - &quantity;

                // let buyer_id = buyer_account_id.clone();

                {
                    self.products.replace(product_index as u64, &Product {
                        product_id: product.product_id,
                        name: product.name,
                        ipfs: product.ipfs,
                        price: product.price,
                        total_supply: new_supply,
                        timeout: product.timeout,
                        is_discount: product.is_discount,
                        discount_percent: product.discount_percent,
                        token_amount: product.token_amount,
                        is_reward: product.is_reward,
                        reward_amount: product.reward_amount,
                        time_created: product.time_created,
                        custom: product.custom,
                        user: product.user
                    });
                }
                println!("{:?}", self.products.get(product_index as u64))
            },
            None => panic!("Couldn't find product"),
        }
    }

    pub fn plus_product(
        &mut self,
        product_id: String,
        quantity: u64,
    ) {
        self.assert_only_pipar();

        let product_index = self.products
            .iter()
            .position(|p| p.product_id == product_id)
            .unwrap();

        match self.products.get(product_index as u64) {
            Some(product) => {
                let new_supply = &product.total_supply + &quantity;

                {
                    self.products.replace(product_index as u64, &Product {
                        product_id: product.product_id,
                        name: product.name,
                        ipfs: product.ipfs,
                        price: product.price,
                        total_supply: new_supply,
                        timeout: product.timeout,
                        is_discount: product.is_discount,
                        discount_percent: product.discount_percent,
                        token_amount: product.token_amount,
                        is_reward: product.is_reward,
                        reward_amount: product.reward_amount,
                        time_created: product.time_created,
                        custom: product.custom,
                        user: product.user
                    });
                }
                println!("{:?}", self.products.get(product_index as u64))
            },
            None => panic!("Couldn't find product"),
        }

    }

    pub fn reward_with_token(
        &mut self,
        product_id: String,
        quantity: u128,
        buyer_account_id: AccountId,
    ) -> Promise {
        self.assert_only_pipar();

        let product_index = self.products
            .iter()
            .position(|p| p.product_id == product_id)
            .unwrap();

        match self.products.get(product_index as u64) {
            Some(product) => {
                let token_quantity = &product.reward_amount * &quantity;
                let memo = format!("Thank You for Shopping at {}!", env::current_account_id());
                let current_account = env::current_account_id().to_string();
                let token_account: AccountId = format!("ft.{current_account}").parse().unwrap();

                let storage_args = serde_json::to_vec(&StorageData {
                    account_id: buyer_account_id.clone(),
                    registration_only: false,
                })
                    .unwrap();

                let token_args = serde_json::to_vec(&TokenData {
                    account_id: buyer_account_id.clone(),
                    registration_only: false,
                })
                    .unwrap();

                Promise::new(token_account.clone())
                    .function_call("storage_deposit".to_owned(), storage_args, ONE_YOCTO, CREATE_ACCOUNT)
                    .function_call("ft_transfer".to_owned(), token_args, NO_DEPOSIT, CREATE_ACCOUNT)
                    .then(
                        Self::ext(env::current_account_id())
                            .deploy_token_callback(
                                env::predecessor_account_id(),
                                env::attached_deposit().into(),
                            )
                    )
            },
            None => panic!("Couldn't find product"),
        }

    }

}


