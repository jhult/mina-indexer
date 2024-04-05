use super::gen::Stake;
use crate::ledger::staking::StakingAccount;
use async_graphql::Context;

pub fn to_staking_account(_value: &Stake, _ctx: &Context<'_>) -> StakingAccount {
    // StakingAccount {
    //     nonce: value.nonce,
    //     token: value.token,
    //     timing: value.timing(ctx),
    //     balance: value.balance(ctx),
    //     pk: value.pk,
    //     delegate: value.delegate,
    //     voting_for: value.voting_for,
    //     permissions: value.permissions(ctx),
    //     token_permissions: value.token_permissions,
    //     receipt_chain_hash: value.receipt_chain_hash,
    // }
    //
    panic!("")
}
