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
        _p2: Option<StakeQueryInput>,
        _p3: Option<i64>,
        _p4: Option<StakeSortByInput>,
    ) -> Result<Vec<Option<Stake>>> {
        todo!()
    }

    pub(crate) async fn query_feetransfer(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _query_input: Option<FeetransferQueryInput>,
    ) -> Result<Option<Feetransfer>> {
        todo!()
    }

    pub(crate) async fn query_blocks(
        &self,
        _ctx: &Context<'_>,
        _: &Query,
        _: Option<i64>,
        _sort_by: Option<BlockSortByInput>,
        _query_input: Option<BlockQueryInput>,
    ) -> Result<Vec<Option<Block>>> {
        todo!()
    }

    pub(crate) async fn query_block(
        &self,
        ctx: &Context<'_>,
        _: &Query,
        query: Option<BlockQueryInput>,
    ) -> Result<Option<Block>> {
        let db = db(ctx);
        // Choose genesis block if query is None
        let state_hash = match query {
            Some(query) => BlockHash::from(query.state_hash.expect("State Hash is required")),
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

    pub(crate) async fn mutation_upsert_one_stake(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<StakeQueryInput>,
        _p3: StakeInsertInput,
    ) -> Result<Option<Stake>> {
        todo!()
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

    pub(crate) async fn mutation_insert_many_nextstakes(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Vec<NextstakeInsertInput>,
    ) -> Result<Option<InsertManyPayload>> {
        todo!()
    }

    pub(crate) async fn mutation_insert_many_feetransfers(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Vec<FeetransferInsertInput>,
    ) -> Result<Option<InsertManyPayload>> {
        todo!()
    }

    pub(crate) async fn mutation_delete_many_blocks(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<BlockQueryInput>,
    ) -> Result<Option<DeleteManyPayload>> {
        todo!()
    }

    pub(crate) async fn mutation_replace_one_transaction(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<TransactionQueryInput>,
        _p3: TransactionInsertInput,
    ) -> Result<Option<Transaction>> {
        todo!()
    }

    pub(crate) async fn mutation_delete_one_nextstake(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: NextstakeQueryInput,
    ) -> Result<Option<Nextstake>> {
        todo!()
    }

    pub(crate) async fn mutation_insert_many_blocks(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Vec<BlockInsertInput>,
    ) -> Result<Option<InsertManyPayload>> {
        todo!()
    }

    pub(crate) async fn mutation_delete_one_transaction(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: TransactionQueryInput,
    ) -> Result<Option<Transaction>> {
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

    pub(crate) async fn mutation_upsert_one_transaction(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<TransactionQueryInput>,
        _p3: TransactionInsertInput,
    ) -> Result<Option<Transaction>> {
        todo!()
    }

    pub(crate) async fn mutation_upsert_one_snark(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<SnarkQueryInput>,
        _p3: SnarkInsertInput,
    ) -> Result<Option<Snark>> {
        todo!()
    }

    pub(crate) async fn mutation_upsert_one_nextstake(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: NextstakeInsertInput,
        _p3: Option<NextstakeQueryInput>,
    ) -> Result<Option<Nextstake>> {
        todo!()
    }

    pub(crate) async fn mutation_upsert_one_feetransfer(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<FeetransferQueryInput>,
        _p3: FeetransferInsertInput,
    ) -> Result<Option<Feetransfer>> {
        todo!()
    }

    pub(crate) async fn mutation_upsert_one_block(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<BlockQueryInput>,
        _p3: BlockInsertInput,
    ) -> Result<Option<Block>> {
        todo!()
    }

    pub(crate) async fn mutation_update_one_transaction(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<TransactionQueryInput>,
        _p3: TransactionUpdateInput,
    ) -> Result<Option<Transaction>> {
        todo!()
    }

    pub(crate) async fn mutation_update_one_stake(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<StakeQueryInput>,
        _p3: StakeUpdateInput,
    ) -> Result<Option<Stake>> {
        todo!()
    }

    pub(crate) async fn mutation_update_one_snark(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<SnarkQueryInput>,
        _p3: SnarkUpdateInput,
    ) -> Result<Option<Snark>> {
        todo!()
    }

    pub(crate) async fn mutation_update_one_nextstake(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<NextstakeQueryInput>,
        _p3: NextstakeUpdateInput,
    ) -> Result<Option<Nextstake>> {
        todo!()
    }

    pub(crate) async fn mutation_update_one_feetransfer(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: FeetransferUpdateInput,
        _p3: Option<FeetransferQueryInput>,
    ) -> Result<Option<Feetransfer>> {
        todo!()
    }

    pub(crate) async fn mutation_update_one_block(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<BlockQueryInput>,
        _p3: BlockUpdateInput,
    ) -> Result<Option<Block>> {
        todo!()
    }

    pub(crate) async fn mutation_update_many_transactions(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<TransactionQueryInput>,
        _p3: TransactionUpdateInput,
    ) -> Result<Option<UpdateManyPayload>> {
        todo!()
    }

    pub(crate) async fn mutation_update_many_stakes(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<StakeQueryInput>,
        _p3: StakeUpdateInput,
    ) -> Result<Option<UpdateManyPayload>> {
        todo!()
    }

    pub(crate) async fn mutation_update_many_snarks(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: SnarkUpdateInput,
        _p3: Option<SnarkQueryInput>,
    ) -> Result<Option<UpdateManyPayload>> {
        todo!()
    }

    pub(crate) async fn mutation_update_many_nextstakes(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<NextstakeQueryInput>,
        _p3: NextstakeUpdateInput,
    ) -> Result<Option<UpdateManyPayload>> {
        todo!()
    }

    pub(crate) async fn mutation_update_many_feetransfers(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: FeetransferUpdateInput,
        _p3: Option<FeetransferQueryInput>,
    ) -> Result<Option<UpdateManyPayload>> {
        todo!()
    }

    pub(crate) async fn mutation_update_many_blocks(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<BlockQueryInput>,
        _p3: BlockUpdateInput,
    ) -> Result<Option<UpdateManyPayload>> {
        todo!()
    }

    pub(crate) async fn mutation_replace_one_stake(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<StakeQueryInput>,
        _p3: StakeInsertInput,
    ) -> Result<Option<Stake>> {
        todo!()
    }

    pub(crate) async fn mutation_replace_one_snark(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<SnarkQueryInput>,
        _p3: SnarkInsertInput,
    ) -> Result<Option<Snark>> {
        todo!()
    }

    pub(crate) async fn mutation_replace_one_nextstake(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<NextstakeQueryInput>,
        _p3: NextstakeInsertInput,
    ) -> Result<Option<Nextstake>> {
        todo!()
    }

    pub(crate) async fn mutation_replace_one_feetransfer(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<FeetransferQueryInput>,
        _p3: FeetransferInsertInput,
    ) -> Result<Option<Feetransfer>> {
        todo!()
    }

    pub(crate) async fn mutation_replace_one_block(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<BlockQueryInput>,
        _p3: BlockInsertInput,
    ) -> Result<Option<Block>> {
        todo!()
    }

    pub(crate) async fn mutation_insert_one_transaction(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: TransactionInsertInput,
    ) -> Result<Option<Transaction>> {
        todo!()
    }

    pub(crate) async fn mutation_insert_one_stake(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: StakeInsertInput,
    ) -> Result<Option<Stake>> {
        todo!()
    }

    pub(crate) async fn mutation_insert_one_snark(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: SnarkInsertInput,
    ) -> Result<Option<Snark>> {
        todo!()
    }

    pub(crate) async fn mutation_insert_one_nextstake(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: NextstakeInsertInput,
    ) -> Result<Option<Nextstake>> {
        todo!()
    }

    pub(crate) async fn mutation_insert_one_feetransfer(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: FeetransferInsertInput,
    ) -> Result<Option<Feetransfer>> {
        todo!()
    }

    pub(crate) async fn mutation_insert_one_block(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: BlockInsertInput,
    ) -> Result<Option<Block>> {
        todo!()
    }

    pub(crate) async fn mutation_insert_many_transactions(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Vec<TransactionInsertInput>,
    ) -> Result<Option<InsertManyPayload>> {
        todo!()
    }

    pub(crate) async fn mutation_insert_many_snarks(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Vec<SnarkInsertInput>,
    ) -> Result<Option<InsertManyPayload>> {
        todo!()
    }

    pub(crate) async fn mutation_insert_many_stakes(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Vec<StakeInsertInput>,
    ) -> Result<Option<InsertManyPayload>> {
        todo!()
    }

    pub(crate) async fn mutation_delete_one_stake(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: StakeQueryInput,
    ) -> Result<Option<Stake>> {
        todo!()
    }

    pub(crate) async fn mutation_delete_one_snark(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: SnarkQueryInput,
    ) -> Result<Option<Snark>> {
        todo!()
    }

    pub(crate) async fn mutation_delete_one_feetransfer(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: FeetransferQueryInput,
    ) -> Result<Option<Feetransfer>> {
        todo!()
    }

    pub(crate) async fn mutation_delete_one_block(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: BlockQueryInput,
    ) -> Result<Option<Block>> {
        todo!()
    }

    pub(crate) async fn mutation_delete_many_transactions(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<TransactionQueryInput>,
    ) -> Result<Option<DeleteManyPayload>> {
        todo!()
    }

    pub(crate) async fn mutation_delete_many_stakes(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<StakeQueryInput>,
    ) -> Result<Option<DeleteManyPayload>> {
        todo!()
    }

    pub(crate) async fn mutation_delete_many_snarks(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<SnarkQueryInput>,
    ) -> Result<Option<DeleteManyPayload>> {
        todo!()
    }

    pub(crate) async fn mutation_delete_many_feetransfers(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<FeetransferQueryInput>,
    ) -> Result<Option<DeleteManyPayload>> {
        todo!()
    }

    pub(crate) async fn mutation_delete_many_nextstakes(
        &self,
        _ctx: &Context<'_>,
        _p1: &Mutation,
        _p2: Option<NextstakeQueryInput>,
    ) -> Result<Option<DeleteManyPayload>> {
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
        query: Option<TransactionQueryInput>,
        limit: Option<i64>,
        sort_by: Option<TransactionSortByInput>,
    ) -> Result<Vec<Option<Transaction>>> {
        let db = db(ctx);
        let limit = limit.unwrap_or(100);
        let limit_idx = limit as usize;

        let mut transactions: Vec<Option<Transaction>> = Vec::new();

        let iter = if let Some(ref query_input) = query {
            if let Some(date_time_gte) = query_input.date_time_gte {
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
            }
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

        if let Some(sort_by) = sort_by {
            match sort_by {
                TransactionSortByInput::NonceAsc => {
                    transactions.sort_by(|Some(a), Some(b)| a.nonce.unwrap().cmp(&b.nonce.unwrap()))
                }
                _ => {
                    transactions.sort_by(|Some(a), Some(b)| b.nonce.unwrap().cmp(&a.nonce.unwrap()))
                }
            }
        }

        //Ok(Some(transactions))
        panic!();
    }

    pub(crate) async fn query_transaction(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _p2: Option<TransactionQueryInput>,
    ) -> Result<Option<Transaction>> {
        todo!()
    }

    pub(crate) async fn query_stake(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _p2: Option<StakeQueryInput>,
    ) -> Result<Option<Stake>> {
        todo!()
    }

    pub(crate) async fn query_snarks(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _p2: Option<SnarkQueryInput>,
        _p3: Option<i64>,
        _p4: Option<SnarkSortByInput>,
    ) -> Result<Vec<Option<Snark>>> {
        todo!()
    }

    pub(crate) async fn query_snark(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _p2: Option<SnarkQueryInput>,
    ) -> Result<Option<Snark>> {
        todo!()
    }

    pub(crate) async fn query_nextstakes(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _p2: Option<NextstakeSortByInput>,
        _p3: Option<NextstakeQueryInput>,
        _p4: Option<i64>,
    ) -> Result<Vec<Option<Nextstake>>> {
        todo!()
    }

    pub(crate) async fn query_nextstake(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _p2: Option<NextstakeQueryInput>,
    ) -> Result<Option<Nextstake>> {
        todo!()
    }

    pub(crate) async fn query_feetransfers(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _p2: Option<FeetransferQueryInput>,
        _p3: Option<i64>,
        _p4: Option<FeetransferSortByInput>,
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
