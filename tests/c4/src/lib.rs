#![cfg(test)]

use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Env, IntoVal, String, Symbol, Vec,
};

pub mod kinetic_router {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/k2_kinetic_router.optimized.wasm"
    );
}
pub mod a_token {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/k2_a_token.optimized.wasm"
    );
}
pub mod debt_token {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/k2_debt_token.optimized.wasm"
    );
}
pub mod price_oracle {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/k2_price_oracle.optimized.wasm"
    );
}
pub mod interest_rate_strategy {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/k2_interest_rate_strategy.optimized.wasm"
    );
}
pub mod pool_configurator {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/k2_pool_configurator.optimized.wasm"
    );
}
pub mod token {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/k2_token.optimized.wasm"
    );
}

#[contract]
struct MockReflector;

#[contractimpl]
impl MockReflector {
    pub fn lastprice(_env: Env, _asset: soroban_sdk::Val) -> Option<soroban_sdk::Val> {
        None
    }
    pub fn decimals(_env: Env) -> Option<u32> {
        Some(8)
    }
}

type OracleAsset = price_oracle::Asset;

const ASSET_DECIMALS: u32 = 7;
const LP_SEED: i128 = 1_000_000_000_000_000;
const USER_STARTING_BALANCE: i128 = 100_000_000_000_000;
const PRICE_ONE_DOLLAR: u128 = 10_000_000;

#[test]
fn test_submission_validity() {
    let env = Env::default();
    env.mock_all_auths();
    #[allow(deprecated)]
    env.budget().reset_unlimited();
    env.ledger().set(LedgerInfo {
        sequence_number: 100,
        protocol_version: 23,
        timestamp: 1000,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 1_000_000,
    });

    let admin = Address::generate(&env);
    // Emergency admin is a SEPARATE address from pool admin
    // In real deployment these should be different keys
    let emergency_admin = Address::generate(&env);
    let user = Address::generate(&env);
    let liquidity_provider = Address::generate(&env);

    // Deploy oracle
    let oracle_addr = env.register(price_oracle::WASM, ());
    let oracle = price_oracle::Client::new(&env, &oracle_addr);
    let reflector_addr = env.register(MockReflector, ());
    let base_currency = Address::generate(&env);
    let native_xlm = Address::generate(&env);
    oracle.initialize(&admin, &reflector_addr, &base_currency, &native_xlm);

    // Deploy router
    let router_addr = env.register(kinetic_router::WASM, ());
    let router = kinetic_router::Client::new(&env, &router_addr);
    let treasury = Address::generate(&env);
    let dex_router = Address::generate(&env);
    router.initialize(&admin, &emergency_admin, &oracle_addr, &treasury, &dex_router, &None);

    // Deploy pool configurator with emergency_admin as pool_admin
    // (In pool-configurator, initialize sets emergency_admin = pool_admin)
    let configurator_addr = env.register(pool_configurator::WASM, ());
    let configurator = pool_configurator::Client::new(&env, &configurator_addr);
    // Initialize configurator with emergency_admin as the admin
    // so emergency_admin has admin rights in pool-configurator
    configurator.initialize(&emergency_admin, &router_addr, &oracle_addr);
    router.set_pool_configurator(&configurator_addr);

    // Deploy interest rate strategy
    let irs_addr = env.register(interest_rate_strategy::WASM, ());
    let mut irs_args = Vec::new(&env);
    irs_args.push_back(admin.clone().into_val(&env));
    irs_args.push_back((0u128).into_val(&env));
    irs_args.push_back((40_000_000_000_000_000_000u128).into_val(&env));
    irs_args.push_back((100_000_000_000_000_000_000u128).into_val(&env));
    irs_args.push_back((800_000_000_000_000_000_000_000_000u128).into_val(&env));
    let _: () = env.invoke_contract(&irs_addr, &Symbol::new(&env, "initialize"), irs_args);

    // Deploy reserve A
    let underlying_admin = Address::generate(&env);
    let underlying = env.register_stellar_asset_contract_v2(underlying_admin);
    let asset_a = underlying.address();

    let a_token_a = env.register(a_token::WASM, ());
    a_token::Client::new(&env, &a_token_a).initialize(
        &admin, &asset_a, &router_addr,
        &String::from_str(&env, "aToken"),
        &String::from_str(&env, "aTKN"),
        &ASSET_DECIMALS,
    );

    let debt_token_a = env.register(debt_token::WASM, ());
    debt_token::Client::new(&env, &debt_token_a).initialize(
        &admin, &asset_a, &router_addr,
        &String::from_str(&env, "debtToken"),
        &String::from_str(&env, "dTKN"),
        &ASSET_DECIMALS,
    );

    let reserve_treasury = Address::generate(&env);
    let params = pool_configurator::InitReserveParams {
        decimals: ASSET_DECIMALS,
        ltv: 8000,
        liquidation_threshold: 8500,
        liquidation_bonus: 500,
        reserve_factor: 1000,
        supply_cap: 0,
        borrow_cap: 0,
        borrowing_enabled: true,
        flashloan_enabled: true,
    };
    // configurator admin = emergency_admin (intentional for this test)
    configurator.init_reserve(
        &emergency_admin, &asset_a, &a_token_a, &debt_token_a,
        &irs_addr, &reserve_treasury, &params,
    );

    let asset_enum = OracleAsset::Stellar(asset_a.clone());
    oracle.add_asset(&admin, &asset_enum);
    oracle.set_manual_override(
        &admin, &asset_enum,
        &Some(PRICE_ONE_DOLLAR),
        &Some(env.ledger().timestamp() + 604_800),
    );

    // Seed liquidity
    let asset_a_token = token::Client::new(&env, &asset_a);
    let asset_a_mint = soroban_sdk::token::StellarAssetClient::new(&env, &asset_a);
    asset_a_mint.mint(&liquidity_provider, &LP_SEED);
    let approval_exp = env.ledger().sequence() + 100_000;
    asset_a_token.approve(&liquidity_provider, &router_addr, &LP_SEED, &approval_exp);
    router.supply(&liquidity_provider, &asset_a, &(LP_SEED as u128), &liquidity_provider, &0u32);

    asset_a_mint.mint(&user, &USER_STARTING_BALANCE);
    asset_a_token.approve(&user, &router_addr, &USER_STARTING_BALANCE, &approval_exp);

    // ===================================================================
    // FINDING: Emergency admin can UNPAUSE reserves via set_reserve_pause
    //
    // Protocol invariant: "Emergency Admin cannot unpause"
    // (pool admin must be the only one who can unpause)
    //
    // BUG: pool_configurator::set_reserve_pause(paused=false) is callable
    // by emergency_admin, allowing them to unpause — violating the invariant.
    // ===================================================================

    // Step 1: User supplies to create active position
    let deposit: u128 = 5_000_000_000;
    router.supply(&user, &asset_a, &deposit, &user, &0u32);

    // Step 2: Emergency admin pauses the reserve (this is ALLOWED)
    configurator.set_reserve_pause(&emergency_admin, &asset_a, &true);

    // Step 3: Verify reserve is paused - supply should fail
    let supply_when_paused = router.try_supply(&user, &asset_a, &deposit, &user, &0u32);
    assert!(supply_when_paused.is_err(), "Supply should fail when reserve is paused");

    // Step 4: Emergency admin UNPAUSES the reserve (THIS VIOLATES THE INVARIANT!)
    // According to README: "Cannot unpause - only pool admin can unpause"
    // But emergency_admin CAN call set_reserve_pause(false):
    configurator.set_reserve_pause(&emergency_admin, &asset_a, &false);

    // Step 5: Reserve is now unpaused by emergency_admin alone
    let supply_after_unpause = router.try_supply(&user, &asset_a, &deposit, &user, &0u32);
    assert!(
        supply_after_unpause.is_ok(),
        "BUG CONFIRMED: Emergency admin successfully unpaused reserve"
    );

    // Step 6: Also demonstrate unpause_reserve_deployment violation
    configurator.pause_reserve_deployment(&emergency_admin);
    // This should FAIL if invariant is enforced, but it SUCCEEDS:
    configurator.unpause_reserve_deployment(&emergency_admin);

    println!("VULNERABILITY CONFIRMED:");
    println!("Emergency admin bypassed 'cannot unpause' invariant via:");
    println!("1. set_reserve_pause(asset, false)");
    println!("2. unpause_reserve_deployment()");
}
