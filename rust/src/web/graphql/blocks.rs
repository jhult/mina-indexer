use super::gen::{
    Block, BlockProtocolState, BlockProtocolStateBlockchainState, BlockProtocolStateConsensusState,
    BlockProtocolStateConsensusStateNextEpochDatum,
    BlockProtocolStateConsensusStateNextEpochDatumLedger,
    BlockProtocolStateConsensusStateStakingEpochDatum,
    BlockProtocolStateConsensusStateStakingEpochDatumLedger, DateTime,
};
use crate::{
    block::precomputed::PrecomputedBlock,
    ledger::LedgerHash,
    proof_systems::signer::pubkey::CompressedPubKey,
    protocol::serialization_types::{common::Base58EncodableVersionedType, version_bytes},
    web::graphql::millis_to_rfc_date_string,
};

impl From<PrecomputedBlock> for Block {
    fn from(block: PrecomputedBlock) -> Self {
        let winner_account = block.block_creator();
        let date_time = millis_to_rfc_date_string(block.timestamp().try_into().unwrap());
        let pk_creator = block.consensus_state().block_creator;
        let creator = CompressedPubKey::from(&pk_creator).into_address();
        let scheduled_time = block.scheduled_time.clone();
        let received_time = millis_to_rfc_date_string(scheduled_time.parse::<i64>().unwrap());
        let previous_state_hash = block.previous_state_hash().0;
        let tx_fees = block.tx_fees();
        let snark_fees = block.snark_fees();
        let utc_date = block
            .protocol_state
            .body
            .t
            .t
            .blockchain_state
            .t
            .t
            .timestamp
            .t
            .t
            .to_string();

        let blockchain_state = block.protocol_state.body.t.t.blockchain_state.clone().t.t;
        let snarked_ledger_hash =
            LedgerHash::from_hashv1(blockchain_state.clone().snarked_ledger_hash).0;
        let staged_ledger_hashv1 = blockchain_state
            .staged_ledger_hash
            .t
            .t
            .non_snark
            .t
            .ledger_hash;
        let staged_ledger_hash = LedgerHash::from_hashv1(staged_ledger_hashv1).0;

        // consensus state
        let consensus_state = block.protocol_state.body.t.t.consensus_state.clone().t.t;

        let total_currency = consensus_state.total_currency.t.t;
        let blockchain_length = block.blockchain_length;
        let block_height = blockchain_length;
        let epoch_count = consensus_state.epoch_count.t.t;
        let epoch = epoch_count;
        let has_ancestor_in_same_checkpoint_window =
            consensus_state.has_ancestor_in_same_checkpoint_window;
        let last_vrf_output = block.last_vrf_output();
        let min_window_density = consensus_state.min_window_density.t.t;
        let slot_since_genesis = consensus_state.global_slot_since_genesis.t.t;
        let slot = consensus_state.curr_global_slot.t.t.slot_number.t.t;

        // NextEpochData
        let seed_hashv1 = consensus_state.next_epoch_data.t.t.seed;
        let seed_bs58: Base58EncodableVersionedType<{ version_bytes::EPOCH_SEED }, _> =
            seed_hashv1.into();
        let seed = seed_bs58.to_base58_string().expect("bs58 encoded seed");
        let epoch_length = consensus_state.next_epoch_data.t.t.epoch_length.t.t;

        let start_checkpoint_hashv1 = consensus_state.next_epoch_data.t.t.start_checkpoint;
        let start_checkpoint_bs58: Base58EncodableVersionedType<{ version_bytes::STATE_HASH }, _> =
            start_checkpoint_hashv1.into();
        let start_checkpoint = start_checkpoint_bs58
            .to_base58_string()
            .expect("bs58 encoded start checkpoint");

        let lock_checkpoint_hashv1 = consensus_state.next_epoch_data.t.t.lock_checkpoint;
        let lock_checkpoint_bs58: Base58EncodableVersionedType<{ version_bytes::STATE_HASH }, _> =
            lock_checkpoint_hashv1.into();
        let lock_checkpoint = lock_checkpoint_bs58
            .to_base58_string()
            .expect("bs58 encoded lock checkpoint");

        let ledger_hashv1 = consensus_state.next_epoch_data.t.t.ledger.t.t.hash;
        let ledger_hash_bs58: Base58EncodableVersionedType<{ version_bytes::LEDGER_HASH }, _> =
            ledger_hashv1.into();
        let ledger_hash = ledger_hash_bs58
            .to_base58_string()
            .expect("bs58 encoded ledger hash");
        let ledger_total_currency = consensus_state
            .next_epoch_data
            .t
            .t
            .ledger
            .t
            .t
            .total_currency
            .t
            .t;

        // StakingEpochData
        let staking_seed_hashv1 = consensus_state.staking_epoch_data.t.t.seed;
        let staking_seed_bs58: Base58EncodableVersionedType<{ version_bytes::EPOCH_SEED }, _> =
            staking_seed_hashv1.into();
        let staking_seed = staking_seed_bs58
            .to_base58_string()
            .expect("bs58 encoded seed");

        let staking_epoch_length = consensus_state.staking_epoch_data.t.t.epoch_length.t.t;

        let staking_start_checkpoint_hashv1 =
            consensus_state.staking_epoch_data.t.t.start_checkpoint;
        let staking_start_checkpoint_bs58: Base58EncodableVersionedType<
            { version_bytes::STATE_HASH },
            _,
        > = staking_start_checkpoint_hashv1.into();
        let staking_start_checkpoint = staking_start_checkpoint_bs58
            .to_base58_string()
            .expect("bs58 encoded start checkpoint");

        let staking_lock_checkpoint_hashv1 = consensus_state.staking_epoch_data.t.t.lock_checkpoint;
        let staking_lock_checkpoint_bs58: Base58EncodableVersionedType<
            { version_bytes::STATE_HASH },
            _,
        > = staking_lock_checkpoint_hashv1.into();
        let staking_lock_checkpoint = staking_lock_checkpoint_bs58
            .to_base58_string()
            .expect("bs58 encoded lock checkpoint");

        let staking_ledger_hashv1 = consensus_state.staking_epoch_data.t.t.ledger.t.t.hash;
        let staking_ledger_hash_bs58: Base58EncodableVersionedType<
            { version_bytes::LEDGER_HASH },
            _,
        > = staking_ledger_hashv1.into();
        let staking_ledger_hash = staking_ledger_hash_bs58
            .to_base58_string()
            .expect("bs58 encoded ledger hash");
        let staking_ledger_total_currency = consensus_state
            .staking_epoch_data
            .t
            .t
            .ledger
            .t
            .t
            .total_currency
            .t
            .t;

        Block {
            block_height: block.blockchain_length as i64,
            canonical: todo!(),
            creator: Some(creator),
            date_time: DateTime(date_time),
            protocol_state: BlockProtocolState {
                previous_state_hash,
                blockchain_state: BlockProtocolStateBlockchainState {
                    date: Some(utc_date.clone() as i64),
                    utc_date: Some(utc_date),
                    snarked_ledger_hash,
                    staged_ledger_hash,
                },
                consensus_state: BlockProtocolStateConsensusState {
                    total_currency: total_currency as f64,
                    blockchain_length: Some(blockchain_length as i64),
                    block_height: Some(block_height as i64),
                    epoch: epoch as i64,
                    epoch_count: Some(epoch_count as i64),
                    has_ancestor_in_same_checkpoint_window: Some(
                        has_ancestor_in_same_checkpoint_window,
                    ),
                    last_vrf_output: Some(last_vrf_output),
                    min_window_density: Some(min_window_density as i64),
                    slot: slot as i64,
                    slot_since_genesis: slot_since_genesis as i64,
                    next_epoch_data: Some(BlockProtocolStateConsensusStateNextEpochDatum {
                        seed: Some(seed),
                        epoch_length: Some(epoch_length as i64),
                        start_checkpoint: Some(start_checkpoint),
                        lock_checkpoint: Some(lock_checkpoint),
                        ledger: Some(BlockProtocolStateConsensusStateNextEpochDatumLedger {
                            hash: Some(ledger_hash),
                            total_currency: Some(ledger_total_currency as f64),
                        }),
                    }),
                    staking_epoch_data: Some(BlockProtocolStateConsensusStateStakingEpochDatum {
                        seed: Some(staking_seed),
                        epoch_length: Some(staking_epoch_length as i64),
                        start_checkpoint: Some(staking_start_checkpoint),
                        lock_checkpoint: Some(staking_lock_checkpoint),
                        ledger: Some(BlockProtocolStateConsensusStateStakingEpochDatumLedger {
                            hash: Some(staking_ledger_hash),
                            total_currency: Some(staking_ledger_total_currency as f64),
                        }),
                    }),
                },
            },
            received_time: Some(DateTime(received_time)),
            snark_fees: snark_fees.to_string(),
            snark_jobs: todo!(),
            state_hash: block.state_hash,
            state_hash_field: todo!(),
            transactions: todo!(),
            tx_fees: tx_fees.to_string(),
            winner_account: todo!(),
        }
    }
}
