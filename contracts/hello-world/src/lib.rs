#![allow(non_snake_case)]
#![no_std]
use soroban_sdk::{contract, contracttype, contractimpl, log, Env, Address, Symbol, symbol_short};

// Structure to track faucet statistics
#[contracttype]
#[derive(Clone)]
pub struct FaucetStats {
    pub total_requests: u64,
    pub total_distributed: i128,
    pub active_users: u64,
}

// Structure to track user requests
#[contracttype]
#[derive(Clone)]
pub struct UserRequest {
    pub address: Address,
    pub last_request_time: u64,
    pub total_received: i128,
    pub request_count: u64,
}

// Storage keys
const FAUCET_STATS: Symbol = symbol_short!("F_STATS");
const ADMIN: Symbol = symbol_short!("ADMIN");
const DRIP_AMOUNT: Symbol = symbol_short!("DRIP_AMT");
const COOLDOWN_PERIOD: Symbol = symbol_short!("COOLDOWN");

// Mapping user address to their request history
#[contracttype]
pub enum UserBook {
    User(Address)
}

#[contract]
pub struct TestnetFaucet;

#[contractimpl]
impl TestnetFaucet {
    
    // Initialize the faucet with admin and configuration
    pub fn initialize(env: Env, admin: Address, drip_amount: i128, cooldown_seconds: u64) {
        // Ensure faucet is not already initialized
        if env.storage().instance().has(&ADMIN) {
            log!(&env, "Faucet already initialized");
            panic!("Faucet already initialized");
        }
        
        // Store admin address
        env.storage().instance().set(&ADMIN, &admin);
        
        // Store drip amount (amount of XLM per request)
        env.storage().instance().set(&DRIP_AMOUNT, &drip_amount);
        
        // Store cooldown period (time between requests)
        env.storage().instance().set(&COOLDOWN_PERIOD, &cooldown_seconds);
        
        // Initialize faucet stats
        let stats = FaucetStats {
            total_requests: 0,
            total_distributed: 0,
            active_users: 0,
        };
        env.storage().instance().set(&FAUCET_STATS, &stats);
        
        env.storage().instance().extend_ttl(5000, 5000);
        log!(&env, "Faucet initialized successfully");
    }
    
    // Request testnet XLM from the faucet
    pub fn request_tokens(env: Env, user: Address) -> i128 {
        user.require_auth();
        
        let current_time = env.ledger().timestamp();
        let cooldown: u64 = env.storage().instance().get(&COOLDOWN_PERIOD).unwrap_or(86400); // Default 24 hours
        let drip_amount: i128 = env.storage().instance().get(&DRIP_AMOUNT).unwrap_or(10_0000000); // Default 100 XLM
        
        // Get user's request history
        let mut user_request = Self::get_user_request(env.clone(), user.clone());
        
        // Check if user is within cooldown period
        if user_request.last_request_time > 0 {
            let time_since_last_request = current_time - user_request.last_request_time;
            if time_since_last_request < cooldown {
                log!(&env, "Cooldown period not over. Try again later.");
                panic!("Cooldown period not over");
            }
        }
        
        // Update user request data
        let is_new_user = user_request.request_count == 0;
        user_request.address = user.clone();
        user_request.last_request_time = current_time;
        user_request.total_received += drip_amount;
        user_request.request_count += 1;
        
        // Update faucet stats
        let mut stats = Self::get_faucet_stats(env.clone());
        stats.total_requests += 1;
        stats.total_distributed += drip_amount;
        if is_new_user {
            stats.active_users += 1;
        }
        
        // Store updated data
        env.storage().instance().set(&UserBook::User(user.clone()), &user_request);
        env.storage().instance().set(&FAUCET_STATS, &stats);
        
        env.storage().instance().extend_ttl(5000, 5000);
        
        log!(&env, "Tokens distributed: {} to user", drip_amount);
        drip_amount
    }
    
    // Get faucet statistics
    pub fn get_faucet_stats(env: Env) -> FaucetStats {
        env.storage().instance().get(&FAUCET_STATS).unwrap_or(FaucetStats {
            total_requests: 0,
            total_distributed: 0,
            active_users: 0,
        })
    }
    
    // Get user request history
    pub fn get_user_request(env: Env, user: Address) -> UserRequest {
        let key = UserBook::User(user.clone());
        env.storage().instance().get(&key).unwrap_or(UserRequest {
            address: user,
            last_request_time: 0,
            total_received: 0,
            request_count: 0,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{Address, Env};

    #[test]
    fn test_faucet_initialization() {
        let env = Env::default();
        let contract_id = env.register_contract(None, TestnetFaucet);
        let client = TestnetFaucetClient::new(&env, &contract_id);
        
        let admin = Address::generate(&env);
        let drip_amount: i128 = 100_0000000; // 100 XLM
        let cooldown: u64 = 86400; // 24 hours
        
        client.initialize(&admin, &drip_amount, &cooldown);
        
        let stats = client.get_faucet_stats();
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.total_distributed, 0);
    }

    #[test]
    fn test_request_tokens() {
        let env = Env::default();
        let contract_id = env.register_contract(None, TestnetFaucet);
        let client = TestnetFaucetClient::new(&env, &contract_id);
        
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let drip_amount: i128 = 100_0000000;
        let cooldown: u64 = 60; // 1 minute for testing
        
        client.initialize(&admin, &drip_amount, &cooldown);
        
        env.mock_all_auths();
        let received = client.request_tokens(&user);
        
        assert_eq!(received, drip_amount);
        
        let stats = client.get_faucet_stats();
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.total_distributed, drip_amount);
    }
}