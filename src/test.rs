#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Accounts, Ledger, LedgerInfo};
use soroban_sdk::{vec, AccountId, Env, IntoVal};

soroban_sdk::contractimport!(
    file = "target/wasm32-unknown-unknown/release/soroban_token_contract.wasm"
);

type TokenClient = Client;

fn create_token_contract(e: &Env, admin: &AccountId) -> (BytesN<32>, TokenClient) {
    e.install_contract_wasm(WASM);

    let id = e.register_contract_wasm(None, WASM);
    let token = TokenClient::new(e, &id);
    // decimals, name, symbol don't matter in tests
    token.initialize(
        &Identifier::Account(admin.clone()),
        &7u32,
        &"name".into_val(e),
        &"symbol".into_val(e),
    );
    (id, token)
}

fn create_distribution_contract(e: &Env) -> DistributionContractClient {
    let distr = DistributionContractClient::new(e, e.register_contract(None, DistributionContract {}));
    distr.initialize(&200);
    distr
}

struct DistributionTest {
    env: Env,
    attendee_users: [AccountId; 3],
    token: TokenClient,
    token_id: BytesN<32>,
    contract: DistributionContractClient,
    contract_id: Identifier,
}

impl DistributionTest {

    fn setup() -> Self {
        let env: Env = Default::default();
        env.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 1,
            sequence_number: 10,
            network_passphrase: Default::default(),
            base_reserve: 10,
        });

        let attendee_users = [
            env.accounts().generate(),
            env.accounts().generate(),
            env.accounts().generate(),
        ];

        let token_admin = env.accounts().generate();

        let (token_id, token) = create_token_contract(&env, &token_admin);
        for attendee in attendee_users.clone() {
            token.with_source_account(&token_admin).mint(
                &Signature::Invoker,
                &0,
                &Identifier::Account(attendee.clone()),
                &1000,
            );
        }

        let contract = create_distribution_contract(&env);
        let contract_id = Identifier::Contract(contract.contract_id.clone());
        DistributionTest {
            env,
            attendee_users,
            token,
            token_id,
            contract,
            contract_id,
        }
    }

    fn deposit(&self, attendee: &Identifier) {
        self.call_deposit(
            &self.token_id, &attendee
        );
    }

    fn attend(&self, attendee: &Identifier) {
        self.call_attend(attendee);
    }

    fn withdraw(&self) {
        self.call_withdraw(self.token_id.clone());
    }

    fn call_deposit(
        &self,
        token: &BytesN<32>,
        attendee: &Identifier,
    ) {
        self.contract.deposit(token, attendee);
    }

    fn account_id_to_identifier(&self, account_id: &AccountId) -> Identifier {
        Identifier::Account(account_id.clone())
    }

    fn call_withdraw(
        &self,
        token_id: BytesN<32>
    ) {
        self.contract.withdraw(&token_id);
    }

    fn call_attend(
        &self,
        attendee: &Identifier
    ) {
        self.contract.attend(attendee);
    }

    fn approve_deposit(&self, amount: u32, user: AccountId) {
        self.token
            .with_source_account(&user)
            .incr_allow(
                &Signature::Invoker,
                &0,
                &Identifier::Contract(self.contract.contract_id.clone()),
                &(amount as i128),
            )
    }

}

#[test]
fn test_deposit_attend_and_claim() {
    let test = DistributionTest::setup();

    test.approve_deposit(200, test.attendee_users[0].clone());
    test.approve_deposit(200, test.attendee_users[1].clone());

    // has balance
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[0])),
        1000
    );
    test.deposit(
        &test.account_id_to_identifier(&test.attendee_users[0])
    );
    test.deposit(
        &test.account_id_to_identifier(&test.attendee_users[1])
    );

    // balance decreased
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[0])),
        800
    );
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[1])),
        800
    );

    // User0 attends, but User1 doesn't
    test.attend(
        &test.account_id_to_identifier(&test.attendee_users[0])
    );

    // balance doesn't change
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[0])),
        800
    );
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[1])),
        800
    );

    // withdraw, everything goes to User1
    test.withdraw();

    // balance doesn't change
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[0])),
        1200
    );
    assert_eq!(
        test.token
        .balance(&test.account_id_to_identifier(&test.attendee_users[1])),
        800
    );


}