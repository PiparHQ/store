use near_sdk::{
    self, assert_one_yocto,
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::Vector,
};
use near_sdk::{
    env, is_promise_success, json_types::U128, near_bindgen, PanicOnDefault, AccountId, Balance, Gas,
    Promise,
};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::serde_json::{json};

// Constants
pub const ONE_NEAR: u128 = 1_000_000_000_000_000_000_000_000;
pub const TOKEN_BALANCE: u128 = 4_000_000_000_000_000_000_000_000;
pub const NO_DEPOSIT: Balance = 0;
pub const ONE_YOCTO: u128 = 10_000_000_000_000_000_000_000;

pub const fn tgas(n: u64) -> Gas {
    Gas(n * 10u64.pow(12))
}

pub(crate) fn yocto_to_near(yocto: u128) -> f64 {
    //10^20 yoctoNEAR (1 NEAR would be 10_000). This is to give a precision of 4 decimal places.
    let formatted_near = yocto / 100_000_000_000_000_000_000;
    let near = formatted_near as f64 / 10_000_f64;

    near
}

pub const CREATE_ACCOUNT: Gas = tgas(65 + 5);

#[near_bindgen]
#[derive(Debug, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Product {
    product_id: u64,
    name: String,
    ipfs: String,
    price: u128,
    total_supply: u128,
    timeout: u8,
    is_discount: bool,
    discount_percent: u8,
    token_amount: u128,
    is_reward: bool,
    reward_amount: u128,
    time_created: u64,
    custom: bool,
    user: Option<String>,
}

#[near_bindgen]
#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct FtData {
    owner_id: AccountId,
    total_supply: String,
    name: String,
    symbol: String,
    icon: String,
}

#[near_bindgen]
#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct StorageData {
    account_id: AccountId,
    registration_only: bool,
}

#[near_bindgen]
#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct TokenData {
    receiver_id: AccountId,
    amount: u128,
    memo: String,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct PiparStoreFactory {
    pub products: Vector<Product>,
    pub owner_id: AccountId,
    pub contract_id: AccountId,
    pub token: bool,
    pub token_cost: u128,
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
            false, self.token,
            "Store owner has already deployed a token"
        )
    }

    pub fn get_product_count(&self) -> usize {
        self.products.iter().count()
    }

    pub fn get_store_products(&self) {
        // let num: usize = self.products.iter().count();
        // let products= self.products.iter().take(num);
        // println!("{:?}", products)
        for p in self.products.iter() {
            println!("{:?}", p)
        }
    }

    pub fn get_token_cost(&self) -> U128 {
        self.token_cost.into()
    }

    pub fn has_token(&self) -> bool {
        self.token.into()
    }

    #[init]
    pub fn new(owner_id: AccountId, contract_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            products: Vector::new(b"vec-uid-1".to_vec()),
            owner_id,
            contract_id,
            token: false,
            token_cost: TOKEN_BALANCE,
        }
    }

    #[private]
    pub fn deploy_token_callback(&mut self, token_creator_id: AccountId, attached_deposit: U128) {
        let attached_deposit: u128 = attached_deposit.into();
        if is_promise_success() {
            env::log_str("Successful token deployment")
        } else {
            Promise::new(token_creator_id).transfer(attached_deposit);
            env::log_str("failed token deployment")
        }
    }

    #[payable]
    pub fn deploy_token(
        &mut self,
        total_supply: String,
        name: String,
        symbol: String,
        icon: String,
    ) -> Promise {
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
            icon,
        })
        .unwrap();

        Promise::new(subaccount.clone())
            .create_account()
            .add_full_access_key(env::signer_account_pk())
            .transfer(TOKEN_BALANCE)
            .deploy_contract(include_bytes!("../wasm/pipar_fungible_token.wasm").to_vec())
            .function_call(
                "new_default_meta".to_owned(),
                init_args,
                NO_DEPOSIT,
                CREATE_ACCOUNT,
            )
            .then(Self::ext(env::current_account_id()).deploy_token_callback(
                env::predecessor_account_id(),
                env::attached_deposit().into(),
            ))
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
        reward_amount: u128,
        custom: bool,
        user: Option<String>,
    ) {
        self.assert_only_owner();
        let id = env::block_timestamp_ms();
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
            user,
        })
    }

    pub fn store_purchase_product(
        &mut self,
        product_id: u64,
        buyer_account_id: AccountId,
        attached_near: Balance,
    ) {
        self.assert_only_pipar();

        let product_index = self
            .products
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
                    self.products.replace(
                        product_index as u64,
                        &Product {
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
                            user: product.user,
                        },
                    );
                }
                println!("{:?}", self.products.get(product_index as u64))
            }
            None => panic!("Couldn't find product"),
        }
    }

    pub fn plus_product(&mut self, product_id: u64, quantity: u128) {
        self.assert_only_pipar();

        let product_index = self
            .products
            .iter()
            .position(|p| p.product_id == product_id)
            .unwrap();

        match self.products.get(product_index as u64) {
            Some(product) => {
                let new_supply = &product.total_supply + &quantity;

                {
                    self.products.replace(
                        product_index as u64,
                        &Product {
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
                            user: product.user,
                        },
                    );
                }
                println!("{:?}", self.products.get(product_index as u64))
            }
            None => panic!("Couldn't find product"),
        }
    }

    pub fn reward_with_token(
        &mut self,
        product_id: u64,
        quantity: u128,
        buyer_account_id: AccountId,
    ) -> Promise {
        self.assert_only_pipar();

        let product_index = self
            .products
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
                    receiver_id: buyer_account_id.clone(),
                    amount: token_quantity,
                    memo,
                })
                .unwrap();

                Promise::new(token_account.clone())
                    .function_call(
                        "storage_deposit".to_owned(),
                        storage_args,
                        ONE_YOCTO,
                        CREATE_ACCOUNT,
                    )
                    .function_call(
                        "ft_transfer".to_owned(),
                        token_args,
                        NO_DEPOSIT,
                        CREATE_ACCOUNT,
                    )
                    .then(
                        Self::ext(env::current_account_id())
                            .reward_with_token_callback(token_quantity.clone()),
                    )
            }
            None => panic!("Couldn't find product"),
        }
    }

    #[private]
    pub fn reward_with_token_callback(&self, token_quantity: u128) {
        if is_promise_success() {
            println!("Sent {:?} token successfully", token_quantity)
        } else {
            println!("failed sending token")
        }
    }
}
