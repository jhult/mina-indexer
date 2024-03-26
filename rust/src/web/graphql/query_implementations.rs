use super::{db, gen::*, transactions::TX_COLUMN};
use crate::{
    block::{store::BlockStore, BlockHash},
    canonicity::{store::CanonicityStore, Canonicity},
};
use async_graphql::{Context, Result};
use speedb::{Direction, IteratorMode};

pub struct DataSource;

impl DataSource {
    pub(crate) async fn query_stakes(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _input: StakeQueryInput,
        _limit: Option<i64>,
        _sort_by: StakeSortByInput,
    ) -> Result<Vec<Option<Stake>>> {
        todo!()
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

    pub(crate) async fn block_transactions(
        &self,
        _ctx: &Context<'_>,
        _p1: &Block,
    ) -> Result<Option<BlockTransaction>> {
        todo!()
    }

    pub(crate) async fn block_winner_account(
        &self,
        _ctx: &Context<'_>,
        _p1: &Block,
    ) -> Result<Option<BlockWinnerAccount>> {
        todo!()
    }

    pub(crate) async fn nextstake_next_delegation_totals(
        &self,
        _ctx: &Context<'_>,
        _p1: &Nextstake,
    ) -> Result<Option<NextDelegationTotal>> {
        todo!()
    }

    pub(crate) async fn block_transaction_user_command_receiver(
        &self,
        _ctx: &Context<'_>,
        _p1: &BlockTransactionUserCommand,
    ) -> Result<Option<BlockTransactionUserCommandReceiver>> {
        todo!()
    }

    pub(crate) async fn block_snark_jobs(
        &self,
        _ctx: &Context<'_>,
        _p1: &Block,
    ) -> Result<Option<Vec<Option<BlockSnarkJob>>>> {
        todo!()
    }

    pub(crate) async fn block_protocol_state(
        &self,
        _ctx: &Context<'_>,
        _p1: &Block,
    ) -> Result<Option<BlockProtocolState>> {
        todo!()
    }

    pub(crate) async fn block_creator_account(
        &self,
        _ctx: &Context<'_>,
        _p1: &Block,
    ) -> Result<Option<BlockCreatorAccount>> {
        todo!()
    }

    pub(crate) async fn block_protocol_state_consensus_state_staking_epoch_data(
        &self,
        _ctx: &Context<'_>,
        _p1: &BlockProtocolStateConsensusState,
    ) -> Result<Option<BlockProtocolStateConsensusStateStakingEpochDatum>> {
        todo!()
    }

    pub(crate) async fn block_protocol_state_consensus_state_next_epoch_data(
        &self,
        _ctx: &Context<'_>,
        _p1: &BlockProtocolStateConsensusState,
    ) -> Result<Option<BlockProtocolStateConsensusStateNextEpochDatum>> {
        todo!()
    }

    pub(crate) async fn block_protocol_state_consensus_state_next_epoch_datum_ledger(
        &self,
        _ctx: &Context<'_>,
        _p1: &BlockProtocolStateConsensusStateNextEpochDatum,
    ) -> Result<Option<BlockProtocolStateConsensusStateNextEpochDatumLedger>> {
        todo!()
    }

    pub(crate) async fn block_protocol_state_consensus_state(
        &self,
        _ctx: &Context<'_>,
        _p1: &BlockProtocolState,
    ) -> Result<Option<BlockProtocolStateConsensusState>> {
        todo!()
    }

    pub(crate) async fn block_protocol_state_blockchain_state(
        &self,
        _ctx: &Context<'_>,
        _p1: &BlockProtocolState,
    ) -> Result<Option<BlockProtocolStateBlockchainState>> {
        todo!()
    }

    pub(crate) async fn transaction_to_account(
        &self,
        _ctx: &Context<'_>,
        _p1: &Transaction,
    ) -> Result<Option<TransactionToAccount>> {
        todo!()
    }

    pub(crate) async fn transaction_source(
        &self,
        _ctx: &Context<'_>,
        _p1: &Transaction,
    ) -> Result<Option<TransactionSource>> {
        todo!()
    }

    pub(crate) async fn transaction_receiver(
        &self,
        _ctx: &Context<'_>,
        _p1: &Transaction,
    ) -> Result<Option<TransactionReceiver>> {
        todo!()
    }

    pub(crate) async fn transaction_from_account(
        &self,
        _ctx: &Context<'_>,
        _p1: &Transaction,
    ) -> Result<Option<TransactionFromAccount>> {
        todo!()
    }

    pub(crate) async fn transaction_fee_payer(
        &self,
        _ctx: &Context<'_>,
        _p1: &Transaction,
    ) -> Result<Option<TransactionFeePayer>> {
        todo!()
    }

    pub(crate) async fn transaction_block(
        &self,
        _ctx: &Context<'_>,
        _p1: &Transaction,
    ) -> Result<Option<Block>> {
        todo!()
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
        sort_by: TransactionSortByInput,
    ) -> Result<Vec<Option<Transaction>>> {
        let db = db(ctx);
        let limit = limit.unwrap_or(100);
        let limit_idx = limit as usize;

        let mut transactions: Vec<Option<Transaction>> = Vec::new();

        let iter = if let Some(date_time_gte) = input.date_time_gte {
            // TODO: what incoming format and timezone? UTC?
            let bytes = chrono::DateTime::parse_from_rfc2822(&date_time_gte.0)
                .unwrap()
                .timestamp_millis()
                .to_string()
                .into_bytes();
            //let key = Base32Hex.encode(&bytes);

            let mode = IteratorMode::From(&key.into_bytes(), Direction::Forward);
            let mut iter = db.database.iterator_cf(TX_COLUMN.as_ref(), mode);
            iter.set_mode(mode);
            iter
        } else {
            db.database
                .iterator_cf(TX_COLUMN.as_ref(), IteratorMode::Start)
        };

        for entry in iter {
            let (key, value) = entry.unwrap();

            // TODO: fix
            //let key = Transaction {}; //
            // TransactionKey::from_slice(&key).unwrap();

            //let cmd = UserCommandWithStatusV1::new();
            // let cmd = bcs::from_bytes::<UserCommandWithStatusV1>(&value)
            //     .unwrap()
            //     .inner();

            // let transaction = Transaction::from_cmd(
            //     UserCommandWithStatusJson::from(cmd),
            //     key.height() as i32,
            //     key.timestamp(),
            //     key.hash(),
            // );

            // // If query is provided, only add transactions that satisfy the
            // query if let Some(ref query_input) = query {
            //     if query_input.matches(&transaction) {
            //         transactions.push(Some(transaction));
            //     }
            // }
            // // If no query is provided, add all transactions
            // else {
            //     transactions.push(Some(transaction));
            // }
            // // Early break if the transactions reach the query limit
            // if transactions.len() >= limit_idx {
            //     break;
            // }
        }

        match sort_by {
            TransactionSortByInput::NonceAsc => {
                transactions.sort_by(|Some(a), Some(b)| a.nonce.cmp(&b.nonce))
            }
            _ => transactions.sort_by(|Some(a), Some(b)| b.nonce.cmp(&a.nonce)),
        }

        //Ok(Some(transactions))
        panic!();
    }

    pub(crate) async fn query_transaction(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _p2: TransactionQueryInput,
    ) -> Result<Option<Transaction>> {
        todo!()
    }

    pub(crate) async fn query_stake(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _p2: StakeQueryInput,
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
        _p2: FeetransferQueryInput,
        _p3: Option<i64>,
        _p4: FeetransferSortByInput,
    ) -> Result<Vec<Option<Feetransfer>>> {
        todo!()
    }

    pub(crate) async fn block_protocol_state_consensus_state_staking_epoch_datum_ledger(
        &self,
        _ctx: &Context<'_>,
        _p1: &BlockProtocolStateConsensusStateStakingEpochDatum,
    ) -> Result<Option<BlockProtocolStateConsensusStateStakingEpochDatumLedger>> {
        todo!()
    }

    pub(crate) async fn snark_block(
        &self,
        _ctx: &Context<'_>,
        _p1: &Snark,
    ) -> Result<Option<Block>> {
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
        _ctx: &Context<'_>,
        _p1: &Stake,
    ) -> Result<Option<DelegationTotal>> {
        todo!()
    }

    pub(crate) async fn block_winner_account_balance(
        &self,
        _ctx: &Context<'_>,
        _p1: &BlockWinnerAccount,
    ) -> Result<Option<BlockWinnerAccountBalance>> {
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
    ) -> Result<Option<Vec<Option<BlockTransactionUserCommand>>>> {
        todo!()
    }

    pub(crate) async fn block_transaction_fee_transfer(
        &self,
        _ctx: &Context<'_>,
        _p1: &BlockTransaction,
    ) -> Result<Option<Vec<Option<BlockTransactionFeeTransfer>>>> {
        unimplemented!()
    }

    pub(crate) async fn block_transaction_coinbase_receiver_account(
        &self,
        _ctx: &Context<'_>,
        _p1: &BlockTransaction,
    ) -> Result<Option<BlockTransactionCoinbaseReceiverAccount>> {
        todo!()
    }
}
