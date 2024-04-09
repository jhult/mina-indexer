use super::{
    db,
    gen::{
        DelegationTotal, Nextstake, NextstakeQueryInput, NextstakeSortByInput, Query, Stake,
        StakeQueryInput, StakeSortByInput,
    },
    DataSource,
};
use crate::ledger::store::LedgerStore;
use async_graphql::{Context, Result};
use rust_decimal::{prelude::ToPrimitive, Decimal};

impl DataSource {
    pub async fn query_stake(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _input: StakeQueryInput,
    ) -> Result<Option<Stake>> {
        todo!()
    }

    pub async fn query_stakes(
        &self,
        ctx: &Context<'_>,
        _: &Query,
        input: StakeQueryInput,
        limit: Option<i64>,
        sort_by: StakeSortByInput,
    ) -> Result<Vec<Option<Stake>>> {
        let epoch = input.epoch.unwrap_or(0) as u32;
        let limit = limit.unwrap_or(100) as usize;

        let db = db(ctx);

        let staking_ledger = match db.get_staking_ledger_at_epoch("mainnet", epoch)? {
            Some(staking_ledger) => staking_ledger,
            None => return Ok(vec![]),
        };

        // Delegations will be present if the staking ledger is
        let delegations = db.get_delegations_epoch("mainnet", epoch)?.unwrap();

        let mut accounts: Vec<Option<Stake>> = staking_ledger
            .staking_ledger
            .into_values()
            .filter(|account| {
                if let Some(public_key) = &input.public_key {
                    return *public_key == account.pk.0;
                }
                true
            })
            .map(|account| {
                let pk = account.pk.clone();
                let result = delegations.delegations.get(&pk).unwrap();
                let total_delegated_nanomina = result.total_delegated.unwrap_or_default();
                let count_delegates = result.count_delegates.unwrap_or_default();

                let mut decimal = Decimal::from(total_delegated_nanomina);
                decimal.set_scale(9).ok();
                let balance = decimal.to_f64().unwrap_or_default();
                let nonce = account.nonce.unwrap_or_default() as i64;
                let delegate = account.delegate.0;
                let token = account.token as i64;
                let receipt_chain_hash = account.receipt_chain_hash.0;
                let voting_for = account.voting_for.0;

                Some(Stake {
                    balance,
                    // TODO: update for Berkeley
                    chain_id: "5f704c".to_string(),
                    delegate,
                    delegation_totals: DelegationTotal {
                        count_delegates: count_delegates as i64,
                        total_delegated: total_delegated_nanomina as f64,
                    },
                    epoch: epoch.into(),
                    ledger_hash: staking_ledger.ledger_hash.clone().0,
                    nonce,
                    // TODO: fix
                    permissions: None,
                    pk: Some(pk.clone().0),
                    public_key: pk.0,
                    receipt_chain_hash,
                    // TODO: fix
                    timing: None,
                    token,
                    voting_for,
                })
            })
            .collect();

        match sort_by {
            StakeSortByInput::BalanceDesc => {
                accounts.sort_by(|a, b| {
                    if let (Some(a), Some(b)) = (a, b) {
                        a.balance.partial_cmp(&b.balance).unwrap()
                    } else {
                        a.is_none().cmp(&b.is_none())
                    }
                });
            }
            _ => {}
        }
        accounts.truncate(limit);
        Ok(accounts)
    }

    pub(crate) async fn query_nextstake(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _p2: NextstakeQueryInput,
    ) -> Result<Option<Nextstake>> {
        todo!()
    }

    pub(crate) async fn query_nextstakes(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _sort_by: NextstakeSortByInput,
        _input: NextstakeQueryInput,
        _limit: Option<i64>,
    ) -> Result<Vec<Option<Nextstake>>> {
        todo!()
    }
}
