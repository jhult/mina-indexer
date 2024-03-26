use super::gen::Block;
use crate::{
    block::precomputed::PrecomputedBlock,
    proof_systems::signer::pubkey::CompressedPubKey,
    protocol::serialization_types::{
        common::{Base58EncodableVersionedType, HashV1},
        version_bytes,
    },
    web::millis_to_rfc_date_string,
};

pub struct LedgerHash(pub String);

impl LedgerHash {
    pub fn from_hashv1(hashv1: HashV1) -> Self {
        let versioned: Base58EncodableVersionedType<{ version_bytes::LEDGER_HASH }, _> =
            hashv1.into();
        Self(versioned.to_base58_string().unwrap())
    }
}

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
        todo!()
        // Block {
        //     state_hash: block.state_hash,
        //     block_height: block.blockchain_length,
        //     date_time,
        //     winner_account: BlockWinnerAccount { winner_account },
        //     creator_account: CreatorAccount {
        //         public_key: creator.clone(),
        //     },
        //     creator,
        //     received_time,
        //     protocol_state: ProtocolState {
        //         previous_state_hash,
        //         blockchain_state: BlockchainState {
        //             date: utc_date.clone(),
        //             utc_date,
        //             snarked_ledger_hash,
        //             staged_ledger_hash,
        //         },
        //         consensus_state: ConsensusState {
        //             total_currency,
        //             blockchain_length,
        //             block_height,
        //             epoch,
        //             epoch_count,
        //             has_ancestor_in_same_checkpoint_window,
        //             last_vrf_output,
        //             min_window_density,
        //             slot,
        //             slot_since_genesis,
        //         },
        //     },
        //     tx_fees: tx_fees.to_string(),
        //     snark_fees: snark_fees.to_string(),
        // }
    }
}
