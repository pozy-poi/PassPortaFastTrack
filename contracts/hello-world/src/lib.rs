#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, BytesN, token};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Application(Address), // Maps a Traveler's wallet address to their Visa Application state
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VisaApplication {
    pub traveler: Address,
    pub embassy: Address,
    pub token: Address,
    pub fee_amount: i128,
    pub document_hash: BytesN<32>, // Cryptographic hash of the passport/credentials payload
    pub is_approved: bool,
    pub is_completed: bool,
}

#[contract]
pub struct PassPortaContract;

#[contractimpl]
impl PassPortaContract {
    /// Registers a traveler's application data and places the required embassy processing fee into escrow.
    pub fn submit_application(env: Env, traveler: Address, embassy: Address, token: Address, fee_amount: i128, document_hash: BytesN<32>) {
        traveler.require_auth();

        let key = DataKey::Application(traveler.clone());
        if env.storage().persistent().has(&key) {
            panic!("An active visa application pipeline is already running for this traveler");
        }

        // Lock the visa application fee directly inside the contract runtime
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&traveler, &env.current_contract_address(), &fee_amount);

        let application = VisaApplication {
            traveler,
            embassy,
            token,
            fee_amount,
            document_hash,
            is_approved: false,
            is_completed: false,
        };

        env.storage().persistent().set(&key, &application);
    }

    /// Allows an authorized embassy officer to approve the digital application, triggering an immediate fee payout.
    pub fn approve_visa(env: Env, traveler: Address) {
        let key = DataKey::Application(traveler.clone());
        let mut application: VisaApplication = env.storage().persistent().get(&key).unwrap_or_else(|| {
            panic!("Target visa application was not found")
        });

        // Enforce that only the assigned embassy can verify and sign off
        application.embassy.require_auth();

        if application.is_completed {
            panic!("This application pipeline has already been completely finalized");
        }

        // Update state to reflect government approval parameters
        application.is_approved = true;
        application.is_completed = true;

        // Route the locked processing fee directly from escrow to the embassy treasury wallet
        let token_client = token::Client::new(&env, &application.token);
        token_client.transfer(&env.current_contract_address(), &application.embassy, &application.fee_amount);

        env.storage().persistent().set(&key, &application);
    }

    /// Pulls current passport application pipeline states for frontend user tracking dashboards.
    pub fn get_application(env: Env, traveler: Address) -> VisaApplication {
        let key = DataKey::Application(traveler);
        env.storage().persistent().get(&key).unwrap_or_else(|| {
            panic!("No application found")
        })
    }
}
