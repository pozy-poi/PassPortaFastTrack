#![cfg(test)]
use super::*;
use soroban_sdk::{Env, Address, BytesN, token};

fn setup_test_environment(env: &Env) -> (Address, Address, BytesN<32>, Address, token::Client, token::StellarAssetClient) {
    let traveler = Address::generate(env);
    let embassy = Address::generate(env);
    
    // Create a mock document hash representation
    let doc_hash = BytesN::from_array(env, &[1u8; 32]);
    
    // Deploy stablecoin asset (e.g., Global USD or EUR for processing charges)
    let token_id = env.register_stellar_asset_contract(Address::generate(env));
    let token_client = token::Client::new(env, &token_id);
    let token_admin = token::StellarAssetClient::new(env, &token_id);
    
    (traveler, embassy, doc_hash, token_id, token_client, token_admin)
}

#[test]
fn test_happy_path_application_and_approval() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, PassPortaContract);
    let contract_client = PassPortaContractClient::new(&env, &contract_id);

    let (traveler, embassy, doc_hash, token_id, token_client, token_admin) = setup_test_environment(&env);

    // Fund traveler account with stablecoins
    token_admin.mint(&traveler, &500);
    assert_eq!(token_client.balance(&traveler), 500);

    // Step 1: Submit documentation and lock processing fee
    contract_client.submit_application(&traveler, &embassy, &token_id, &200, &doc_hash);
    
    assert_eq!(token_client.balance(&traveler), 300);
    assert_eq!(token_client.balance(&contract_id), 200);

    // Step 2: Embassy verifies records and approves application
    contract_client.approve_visa(&traveler);

    // Verification: Funds transferred to embassy; process completes successfully
    assert_eq!(token_client.balance(&embassy), 200);
    assert_eq!(token_client.balance(&contract_id), 0);

    let check_state = contract_client.get_application(&traveler);
    assert!(check_state.is_approved);
    assert!(check_state.is_completed);
}

#[test]
#[should_panic(expected = "An active visa application pipeline is already running for this traveler")]
fn test_duplicate_application_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, PassPortaContract);
    let contract_client = PassPortaContractClient::new(&env, &contract_id);

    let (traveler, embassy, doc_hash, token_id, _, token_admin) = setup_test_environment(&env);
    token_admin.mint(&traveler, &1000);

    contract_client.submit_application(&traveler, &embassy, &token_id, &200, &doc_hash);
    // Double submission to lock up state must trigger contract panic
    contract_client.submit_application(&traveler, &embassy, &token_id, &200, &doc_hash);
}

#[test]
fn test_state_verification_storage() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, PassPortaContract);
    let contract_client = PassPortaContractClient::new(&env, &contract_id);

    let (traveler, embassy, doc_hash, token_id, _, token_admin) = setup_test_environment(&env);
    token_admin.mint(&traveler, &200);

    contract_client.submit_application(&traveler, &embassy, &token_id, &200, &doc_hash);

    let active_app = contract_client.get_application(&traveler);
    assert_eq!(active_app.fee_amount, 200);
    assert_eq!(active_app.document_hash, doc_hash);
    assert_eq!(active_app.is_completed, false);
}

#[test]
#[should_panic(expected = "Target visa application was not found")]
fn test_approve_non_existent_application_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, PassPortaContract);
    let contract_client = PassPortaContractClient::new(&env, &contract_id);

    let random_traveler = Address::generate(&env);
    contract_client.approve_visa(&random_traveler);
}

#[test]
#[should_panic(expected = "This application pipeline has already been completely finalized")]
fn test_double_approval_protection() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, PassPortaContract);
    let contract_client = PassPortaContractClient::new(&env, &contract_id);

    let (traveler, embassy, doc_hash, token_id, _, token_admin) = setup_test_environment(&env);
    token_admin.mint(&traveler, &200);

    contract_client.submit_application(&traveler, &embassy, &token_id, &200, &doc_hash);
    contract_client.approve_visa(&traveler);
    
    // Executing duplicate approvals onto a finalized entry must abort execution
    contract_client.approve_visa(&traveler);
}