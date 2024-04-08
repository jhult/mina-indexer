use super::{date_time_to_scalar, db, gen::*, stakes::to_staking_account};
use crate::{
    block::{store::BlockStore, BlockHash},
    canonicity::{store::CanonicityStore, Canonicity},
    ledger::{public_key::PublicKey, staking::AggregatedEpochStakeDelegations, store::*},
    store::{
        user_commands_iterator, user_commands_iterator_signed_command,
        user_commands_iterator_txn_hash,
    },
};
use async_graphql::{Context, Result};
use rust_decimal::{prelude::ToPrimitive, Decimal};

pub struct DataSource;

impl DataSource {
    pub(crate) async fn query_stakes(
        &self,
        ctx: &Context<'_>,
        _p1: &Query,
        input: StakeQueryInput,
        _limit: Option<i64>,
        _sort_by: StakeSortByInput,
    ) -> Result<Vec<Option<Stake>>> {
        let epoch = match input.epoch {
            Some(epoch) => epoch,
            None => return Ok(vec![]),
        };

        let db = db(ctx);

        let staking_ledger = match db.get_staking_ledger_at_epoch("mainnet", epoch as u32)? {
            Some(staking_ledger) => staking_ledger,
            None => return Ok(vec![]),
        };

        let accounts: Vec<Option<Stake>> = staking_ledger
            .staking_ledger
            .into_iter()
            .map(|entry| {
                let pk = entry.0;

                let account = entry.1;
                let balance_nanomina = account.balance;
                let mut decimal = Decimal::from(balance_nanomina);
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
                    ledger_hash: staking_ledger.ledger_hash.clone().0,
                    nonce,
                    pk: Some(pk.clone().0),
                    public_key: pk.0,
                    receipt_chain_hash,
                    token,
                    epoch,
                    voting_for,
                })
            })
            .collect();

        Ok(accounts)
    }

    pub(crate) async fn query_feetransfer(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _input: FeetransferQueryInput,
    ) -> Result<Option<Feetransfer>> {
        todo!()
    }

    pub(crate) async fn query_blocks(
        &self,
        _ctx: &Context<'_>,
        _: &Query,
        _limit: Option<i64>,
        _sort_by: BlockSortByInput,
        _input: BlockQueryInput,
    ) -> Result<Vec<Option<Block>>> {
        todo!()
    }

    pub(crate) async fn query_block(
        &self,
        ctx: &Context<'_>,
        _: &Query,
        input: BlockQueryInput,
    ) -> Result<Option<Block>> {
        let db = db(ctx);
        // Choose genesis block if query is None
        let state_hash = match input.state_hash {
            Some(state_hash) => BlockHash::from(state_hash),
            None => match db.get_canonical_hash_at_height(1)? {
                Some(state_hash) => state_hash,
                None => return Ok(None),
            },
        };
        let pcb = match db.get_block(&state_hash)? {
            Some(pcb) => pcb,
            None => return Ok(None),
        };
        let block = Block::from(pcb);
        let _canonical = db
            .get_block_canonicity(&state_hash)?
            .map(|status| matches!(status, Canonicity::Canonical));
        // TODO: update block with canonical value
        Ok(Some(block))
    }

    pub(crate) async fn nextstake_next_delegation_totals(
        &self,
        _ctx: &Context<'_>,
        _p1: &Nextstake,
    ) -> Result<NextDelegationTotal> {
        todo!()
    }

    pub(crate) async fn block_transaction_user_command_receiver(
        &self,
        _ctx: &Context<'_>,
        _p1: &BlockTransactionUserCommand,
    ) -> Result<Option<BlockTransactionUserCommandReceiver>> {
        todo!()
    }

    pub(crate) async fn block_creator_account(
        &self,
        _ctx: &Context<'_>,
        block: &Block,
    ) -> Result<BlockCreatorAccount> {
        Ok(BlockCreatorAccount {
            public_key: block.creator.to_owned().unwrap(),
        })
    }

    pub(crate) async fn feetransfer_block_state_hash(
        &self,
        _ctx: &Context<'_>,
        _p1: &Feetransfer,
    ) -> Result<Option<Block>> {
        todo!()
    }

    pub(crate) async fn query_transactions(
        &self,
        ctx: &Context<'_>,
        _: &Query,
        input: TransactionQueryInput,
        limit: Option<i64>,
        _sort_by: TransactionSortByInput,
    ) -> Result<Vec<Option<Transaction>>> {
        let db = db(ctx);
        let limit = limit.unwrap_or(100);
        let limit_idx = limit as usize;

        let mut transactions: Vec<Option<Transaction>> = Vec::new();

        let iter = user_commands_iterator(db);

        for entry in iter {
            let txn_hash = user_commands_iterator_txn_hash(&entry)?;

            if let Some(__) = input.hash.to_owned() {
                if txn_hash != __ {
                    continue;
                }
            }

            let cmd = user_commands_iterator_signed_command(&entry)?;

            let block_state_hash = cmd.state_hash.to_owned();
            let block_date_time = date_time_to_scalar(cmd.date_time as i64);

            let transaction = Transaction::from_cmd(cmd, block_date_time, &block_state_hash);

            // Only add transactions that satisfy the query
            if input.matches(&transaction) {
                transactions.push(Some(transaction));
            };

            // Early break if the transactions reach the query limit
            if transactions.len() >= limit_idx {
                break;
            }
        }

        // match sort_by {
        //     TransactionSortByInput::NonceAsc => {
        //         transactions.sort_by(|Some(a), Some(b)| a.nonce.cmp(&b.nonce))
        //     }
        //     _ => transactions.sort_by(|Some(a), Some(b)| b.nonce.cmp(&a.nonce)),
        // }

        Ok(transactions)
    }

    pub(crate) async fn query_transaction(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _input: TransactionQueryInput,
    ) -> Result<Option<Transaction>> {
        todo!()
    }

    pub(crate) async fn query_stake(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _input: StakeQueryInput,
    ) -> Result<Option<Stake>> {
        todo!()
    }

    pub(crate) async fn query_snarks(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _input: SnarkQueryInput,
        _limit: Option<i64>,
        _sort_by: SnarkSortByInput,
    ) -> Result<Vec<Option<Snark>>> {
        todo!()
    }

    pub(crate) async fn query_snark(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _input: SnarkQueryInput,
    ) -> Result<Option<Snark>> {
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

    pub(crate) async fn query_nextstake(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _p2: NextstakeQueryInput,
    ) -> Result<Option<Nextstake>> {
        todo!()
    }

    pub(crate) async fn query_feetransfers(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _input: FeetransferQueryInput,
        _limit: Option<i64>,
        _sort_by: FeetransferSortByInput,
    ) -> Result<Vec<Option<Feetransfer>>> {
        todo!()
    }

    pub(crate) async fn snark_block(&self, _ctx: &Context<'_>, _p1: &Snark) -> Result<Block> {
        todo!()
    }

    pub(crate) async fn nextstake_permissions(
        &self,
        _ctx: &Context<'_>,
        _p1: &Nextstake,
    ) -> Result<Option<NextstakePermission>> {
        todo!()
    }

    pub(crate) async fn nextstake_timing(
        &self,
        _ctx: &Context<'_>,
        _p1: &Nextstake,
    ) -> Result<Option<NextstakeTiming>> {
        todo!()
    }

    pub(crate) async fn stake_timing(
        &self,
        _ctx: &Context<'_>,
        _p1: &Stake,
    ) -> Result<Option<StakeTiming>> {
        todo!()
    }

    pub(crate) async fn stake_permissions(
        &self,
        _ctx: &Context<'_>,
        _p1: &Stake,
    ) -> Result<Option<StakePermission>> {
        todo!()
    }

    pub(crate) async fn stake_delegation_totals(
        &self,
        ctx: &Context<'_>,
        stake: &Stake,
    ) -> Result<DelegationTotal> {
        let AggregatedEpochStakeDelegations { delegations, .. } =
            to_staking_account(stake, ctx).aggregate_delegations()?;

        let pk = PublicKey(stake.public_key(ctx).await?);
        let result = delegations.get(&pk).unwrap();
        let total_delegated_nanomina = result.total_delegated.unwrap_or_default();
        let count_delegates = result.count_delegates.unwrap_or_default() as i64;
        let mut decimal = Decimal::from(total_delegated_nanomina);
        decimal.set_scale(9).ok();
        let total_delegated = decimal.to_f64().unwrap_or_default();

        Ok(DelegationTotal {
            count_delegates,
            total_delegated,
        })
    }

    pub(crate) async fn block_winner_account_balance(
        &self,
        _ctx: &Context<'_>,
        _account: &BlockWinnerAccount,
    ) -> Result<BlockWinnerAccountBalance> {
        todo!()
    }

    pub(crate) async fn block_transaction_user_command_source(
        &self,
        _ctx: &Context<'_>,
        _p1: &BlockTransactionUserCommand,
    ) -> Result<Option<BlockTransactionUserCommandSource>> {
        todo!()
    }

    pub(crate) async fn block_transaction_user_command_to_account(
        &self,
        _ctx: &Context<'_>,
        _p1: &BlockTransactionUserCommand,
    ) -> Result<Option<BlockTransactionUserCommandToAccount>> {
        todo!()
    }

    pub(crate) async fn block_transaction_user_command_fee_payer(
        &self,
        _ctx: &Context<'_>,
        _p1: &BlockTransactionUserCommand,
    ) -> Result<Option<BlockTransactionUserCommandFeePayer>> {
        todo!()
    }

    pub(crate) async fn block_transaction_user_command_from_account(
        &self,
        _ctx: &Context<'_>,
        _p1: &BlockTransactionUserCommand,
    ) -> Result<Option<BlockTransactionUserCommandFromAccount>> {
        todo!()
    }

    pub(crate) async fn block_transaction_user_commands(
        &self,
        _ctx: &Context<'_>,
        _p1: &BlockTransaction,
    ) -> Result<Vec<Option<BlockTransactionUserCommand>>> {
        todo!()
    }

    pub(crate) async fn block_transaction_fee_transfer(
        &self,
        _ctx: &Context<'_>,
        _p1: &BlockTransaction,
    ) -> Result<Vec<Option<BlockTransactionFeeTransfer>>> {
        todo!()
    }

    pub(crate) async fn block_transaction_coinbase_receiver_account(
        &self,
        _ctx: &Context<'_>,
        _p1: &BlockTransaction,
    ) -> Result<BlockTransactionCoinbaseReceiverAccount> {
        todo!()
    }
}
