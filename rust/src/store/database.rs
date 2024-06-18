use super::{username::UsernameUpdate, version::IndexerStoreVersion};
use crate::{
    block::{
        precomputed::{PcbVersion, PrecomputedBlock},
        BlockHash,
    },
    chain::{ChainId, Network},
    command::{internal::InternalCommandWithData, signed::SignedCommandWithData},
    event::IndexerEvent,
    ledger::{
        diff::account::PaymentDiff, public_key::PublicKey, staking::StakingAccount,
        username::Username, LedgerHash,
    },
    snark_work::SnarkWorkSummary,
    store::IndexerStore,
    web::graphql::snarks::SnarkWithCanonicity,
};
use redb::TableDefinition;

///////////////////////
// Account store CFs //
///////////////////////

/// `pk -> balance`
pub(crate) const ACCOUNT_BALANCE: TableDefinition<PublicKey, u64> =
    TableDefinition::new("account-balance");

/// CF for sorting account's by balance
/// `{balance}{pk} -> _`
///
/// - `balance`: 8 BE bytes
pub(crate) const ACCOUNT_BALANCE_SORT: TableDefinition<Vec<u8>, u64> =
    TableDefinition::new("account-balance-sort");

/// `state hash -> balance updates`
pub(crate) const ACCOUNT_BALANCE_UPDATES: TableDefinition<PublicKey, Vec<PaymentDiff>> =
    TableDefinition::new("account-balance-updates");

/////////////////////
// Block store CFs //
/////////////////////

/// `state_hash -> block`
pub(crate) const BLOCKS: TableDefinition<BlockHash, PrecomputedBlock> =
    TableDefinition::new("blocks-hash");

/// `state hash -> pcb version`
pub(crate) const BLOCKS_VERSION: TableDefinition<BlockHash, PcbVersion> =
    TableDefinition::new("blocks-version");

/// ```
/// --------------------------------
/// - key: {global_slot}{state_hash}
/// - val: b""
/// where
/// - global_slot: 4 BE bytes
/// - state_hash:  [BlockHash::LEN] bytes
pub(crate) const BLOCKS_GLOBAL_SLOT_SORT: TableDefinition<Vec<u8>, &[u8; 0]> =
    TableDefinition::new("blocks-global-slot-sort");

/// ```
/// ---------------------------------
/// - key: {block_height}{state_hash}
/// - val: b""
/// where
/// - block_height: 4 BE bytes
/// - state_hash:   [BlockHash::LEN] bytes
pub(crate) const BLOCKS_HEIGHT_SORT: TableDefinition<Vec<u8>, &[u8; 0]> =
    TableDefinition::new("blocks-height-sort");

pub(crate) const BLOCK_HEIGHT_TO_GLOBAL_SLOTS: TableDefinition<u32, Vec<u32>> =
    TableDefinition::new("blocks-height-to-slots");

/// CF for storing: global slot -> height
pub(crate) const BLOCK_GLOBAL_SLOT_TO_HEIGHT: TableDefinition<u32, u32> =
    TableDefinition::new("blocks-slot-to-heights");

pub(crate) const BLOCK_PARENT_HASH: TableDefinition<BlockHash, BlockHash> =
    TableDefinition::new("blocks-parent-hash");

pub(crate) const BLOCK_HEIGHT: TableDefinition<BlockHash, u32> =
    TableDefinition::new("blocks-height");

fn block_global_slot_cf(&self) -> &ColumnFamily {
    self.database
        .cf_handle("blocks-global-slot")
        .expect("blocks-global-slot column family exists")
}

pub(crate) const BLOCK_EPOCH: TableDefinition<BlockHash, u32> =
    TableDefinition::new("blocks-epoch");

pub(crate) const BLOCK_GENESIS_STATE_HASH: TableDefinition<BlockHash, BlockHash> =
    TableDefinition::new("blocks-genesis-hash");

pub(crate) const BLOCK_CREATOR: TableDefinition<BlockHash, PublicKey> =
    TableDefinition::new("blocks-creator");

fn block_coinbase_receiver_cf(&self) -> &ColumnFamily {
    self.database
        .cf_handle("blocks-coinbase-receiver")
        .expect("blocks-coinbase-receiver column family exists")
}

pub(crate) const BLOCK_COINBASE_HEIGHT_SORT: TableDefinition<Vec<u8>, &[u8; 0]> =
    TableDefinition::new("coinbase-receiver-height-sort");

pub(crate) const BLOCK_COINBASE_SLOT_SORT: TableDefinition<Vec<u8>, &[u8; 0]> =
    TableDefinition::new("coinbase-receiver-slot-sort");

pub(crate) const BLOCK_CREATOR_HEIGHT_SORT: TableDefinition<Vec<u8>, &[u8; 0]> =
    TableDefinition::new("block-creator-height-sort");

pub(crate) const BLOCK_CREATOR_SLOT_SORT: TableDefinition<Vec<u8>, &[u8; 0]> =
    TableDefinition::new("block-creator-slot-sort");

/// CF for storing blocks at a fixed height:
/// `height -> list of state hashes at height`
///
/// - `list of state hashes at height`: sorted from best to worst
fn blocks_at_height_cf(&self) -> &ColumnFamily {
    self.database
        .cf_handle("blocks-at-length")
        .expect("blocks-at-length column family exists")
}

/// CF for storing blocks at a fixed global slot:
/// `global slot -> list of state hashes at slot`
///
/// - `list of state hashes at slot`: sorted from best to worst
fn blocks_at_global_slot_cf(&self) -> &ColumnFamily {
    self.database
        .cf_handle("blocks-at-slot")
        .expect("blocks-at-slot column family exists")
}

fn block_comparison_cf(&self) -> &ColumnFamily {
    self.database
        .cf_handle("blocks-comparison")
        .expect("blocks-comparison column family exists")
}

////////////////////////////
// User command store CFs //
////////////////////////////

pub(crate) const USER_COMMANDS: TableDefinition<(&str, BlockHash), SignedCommandWithData> =
    TableDefinition::new("user-commands");

pub(crate) const COINBASE_RECEIVERS: TableDefinition<BlockHash, PublicKey> =
    TableDefinition::new("coinbase-receivers");

fn user_commands_pk_cf(&self) -> &ColumnFamily {
    self.database
        .cf_handle("user-commands-pk")
        .expect("user-commands-pk column family exists")
}

fn user_commands_pk_num_cf(&self) -> &ColumnFamily {
    self.database
        .cf_handle("user-commands-pk-num")
        .expect("user-commands-pk-num column family exists")
}

fn user_command_state_hashes_cf(&self) -> &ColumnFamily {
    self.database
        .cf_handle("user-command-state-hashes")
        .expect("user-command-state-hashes column family exists")
}

fn user_commands_cf(&self) -> &ColumnFamily {
    self.database
        .cf_handle("user-commands")
        .expect("user-commands column family exists")
}

fn user_commands_per_block_cf(&self) -> &ColumnFamily {
    self.database
        .cf_handle("user-commands-block")
        .expect("user-commands-block column family exists")
}

fn user_commands_block_order_cf(&self) -> &ColumnFamily {
    self.database
        .cf_handle("user-commands-block-order")
        .expect("user-commands-block-order column family exists")
}

fn user_commands_num_containing_blocks_cf(&self) -> &ColumnFamily {
    self.database
        .cf_handle("user-commands-num-blocks")
        .expect("user-commands-num-blocks column family exists")
}

/// Key-value pairs
/// ```
/// - key: {height}{txn_hash}{state_hash}
/// - val: b""
/// where
/// - height:     4 BE bytes
/// - txn_hash:   [TXN_HASH_LEN] bytes
/// - state_hash: [BlockHash::LEN] bytes
pub(crate) const USER_COMMANDS_HEIGHT_SORT: TableDefinition<Vec<u8>, &[u8; 0]> =
    TableDefinition::new("user-commands-height-sort");

/// Key-value pairs
/// ```
/// - key: {slot}{txn_hash}{state_hash}
/// - val: b""
/// where
/// - slot:       4 BE bytes
/// - txn_hash:   [TXN_HASH_LEN] bytes
/// - state_hash: [BlockHash::LEN] bytes
pub(crate) const USER_COMMANDS_SLOT_SORT: TableDefinition<Vec<u8>, &[u8; 0]> =
    TableDefinition::new("user-commands-slot-sort");

/// Key-value pairs
/// ```
/// - key: txn_hash
/// - val: blockchain_length
/// where
/// - blockchain_length: 4 BE bytes
fn user_commands_txn_hash_to_block_height_cf(&self) -> &ColumnFamily {
    self.database
        .cf_handle("user-commands-to-block-height")
        .expect("user-commands-to-block-height column family exists")
}

/// Key-value pairs
/// ```
/// - key: txn_hash
/// - val: global_slot
/// where
/// - global_slot: 4 BE bytes
fn user_commands_txn_hash_to_global_slot_cf(&self) -> &ColumnFamily {
    self.database
        .cf_handle("user-commands-to-global-slot")
        .expect("user-commands-to-global-slot column family exists")
}

/// Key-value pairs
/// ```
/// - key: {sender}{global_slot}{txn_hash}{state_hash}
/// - val: amount
/// where
/// - sender:      [PublicKey::LEN] bytes
/// - global_slot: 4 BE bytes
/// - txn_hash:    [TX_HASH_LEN] bytes
/// - state_hash:  [BlockHash::LEN] bytes
/// - amount:      8 BE bytes
pub(crate) const TXN_FROM_SLOT_SORT: TableDefinition<Vec<u8>, u32> =
    TableDefinition::new("txn-from-slot-sort");

/// Key-value pairs
/// ```
/// - key: {sender}{block_height}{txn_hash}{state_hash}
/// - val: amount
/// where
/// - sender:       [PublicKey::LEN] bytes
/// - block_height: 4 BE bytes
/// - txn_hash:     [TX_HASH_LEN] bytes
/// - state_hash:   [BlockHash::LEN] bytes
/// - amount:       8 BE bytes
pub(crate) const TXN_FROM_HEIGHT_SORT: TableDefinition<Vec<u8>, u32> =
    TableDefinition::new("txn-from-height-sort");

/// Key-value pairs
/// ```
/// - key: {receiver}{global_slot}{txn_hash}{state_hash}
/// - val: amount
/// where
/// - receiver:    [PublicKey::LEN] bytes
/// - global_slot: 4 BE bytes
/// - txn_hash:    [TX_HASH_LEN] bytes
/// - state_hash:  [BlockHash::LEN] bytes
/// - amount:      8 BE bytes
pub(crate) const TXN_TO_SLOT_SORT: TableDefinition<Vec<u8>, u32> =
    TableDefinition::new("txn-to-slot-sort");

/// Key-value pairs
/// ```
/// - key: {receiver}{block_height}{txn_hash}{state_hash}
/// - val: amount
/// where
/// - receiver:     [PublicKey::LEN] bytes
/// - block_height: 4 BE bytes
/// - txn_hash:     [TX_HASH_LEN] bytes
/// - state_hash:   [BlockHash::LEN] bytes
/// - amount:       8 BE bytes
pub(crate) const TXN_TO_HEIGHT_SORT: TableDefinition<Vec<u8>, u32> =
    TableDefinition::new("txn-to-height-sort");

////////////////////////////////
// Internal command store CFs //
////////////////////////////////

fn internal_commands_cf(&self) -> &ColumnFamily {
    self.database
        .cf_handle("mainnet-internal-commands")
        .expect("mainnet-internal-commands column family exists")
}

pub(crate) const INTERNAL_COMMANDS_SLOT: TableDefinition<
    (u32, &String, usize),
    InternalCommandWithData,
> = TableDefinition::new("internal-commands-global-slot");

//////////////////////////
// Canonicity store CFs //
//////////////////////////

pub(crate) const CANONICITY_LENGTH: TableDefinition<u32, BlockHash> =
    TableDefinition::new("canonicity-length");

pub(crate) const CANONICITY_SLOT: TableDefinition<u32, BlockHash> =
    TableDefinition::new("canonicity-slot");

//////////////////////
// Ledger store CFs //
//////////////////////

pub(crate) const LEDGERS: TableDefinition<LedgerHash, BlockHash> = TableDefinition::new("ledgers");

fn block_ledger_diff_cf(&self) -> &ColumnFamily {
    self.database
        .cf_handle("blocks-ledger-diff")
        .expect("blocks-ledger-diff column family exists")
}

fn block_staged_ledger_hash_cf(&self) -> &ColumnFamily {
    self.database
        .cf_handle("blocks-staged-ledger-hash")
        .expect("blocks-staged-ledger-hash column family exists")
}

/// CF for storing staking ledgers
/// ```
/// - key: {genesis_hash}{epoch}{ledger_hash}
/// - val: staking ledger
/// where
/// - genesis_hash: [BlockHash::LEN] bytes
/// - epoch:        4 BE bytes
/// - ledger_hash:  [TXN_HASH_LEN] bytes
fn staking_ledgers_cf(&self) -> &ColumnFamily {
    self.database
        .cf_handle("staking-ledgers")
        .expect("staking-ledgers column family exists")
}

/// CF for storing staking ledger hashes
/// ```
/// - key: epoch
/// - val: ledger hash
/// where
/// - epoch:        4 BE bytes
/// - ledger hash:  [TXN_HASH_LEN] bytes
fn staking_ledger_epoch_to_hash_cf(&self) -> &ColumnFamily {
    self.database
        .cf_handle("staking-ledger-epoch-to-hash")
        .expect("staking-ledger-epoch-to-hash column family exists")
}

//// CF for storing staking ledger epochs
/// ```
/// - key: ledger hash
/// - val: epoch
/// where
/// - ledger hash: [TXN_HASH_LEN] bytes
/// - epoch:       4 BE bytes
pub(crate) const STAKING_LEDGER_HASH_TO_EPOCH: TableDefinition<LedgerHash, u32> =
    TableDefinition::new("staking-ledger-epoch");

/// CF for storing staking ledger genesis state hashes
/// ```
/// - key: ledger_hash
/// - val: genesis_hash
/// where
/// - ledger_hash:  [TXN_HASH_LEN] bytes
/// - genesis_hash: [BlockHash::LEN] bytes
fn staking_ledger_genesis_hash_cf(&self) -> &ColumnFamily {
    self.database
        .cf_handle("staking-ledger-genesis-hash")
        .expect("staking-ledger-genesis-hash column family exists")
}

/// CF for storing aggregated staking delegations
/// ```
/// - key: {genesis_hash}{epoch}
/// - val: aggregated epoch delegations
/// where
/// - genesis_hash: [BlockHash::LEN] bytes
/// - epoch:        4 BE bytes
fn staking_delegations_cf(&self) -> &ColumnFamily {
    self.database
        .cf_handle("staking-delegations")
        .expect("staking-delegations column family exists")
}

//// Key-value pairs
/// ```
/// - key: {balance}{pk}
/// - val: b""
/// where
/// - balance: 8 BE bytes
/// - pk:      [PublicKey::LEN] bytes
pub(crate) const STAKING_LEDGER_BALANCE: TableDefinition<Vec<u8>, StakingAccount> =
    TableDefinition::new("staking-ledger-balance");

/// Key-value pairs
/// ```
/// - key: {stake}{pk}
/// - val: b""
/// where
/// - stake: 8 BE bytes
/// - pk:    [PublicKey::LEN] bytes
pub(crate) const STAKING_LEDGER_STAKE: TableDefinition<Vec<u8>, StakingAccount> =
    TableDefinition::new("staking-ledger-stake");

/////////////////////
// SNARK store CFs //
/////////////////////

pub(crate) const SNARKS: TableDefinition<BlockHash, Vec<SnarkWorkSummary>> =
    TableDefinition::new("snarks");

/// CF for storing all snark work fee totals
pub(crate) const SNARK_TOP_PRODUCERS: TableDefinition<PublicKey, u64> =
    TableDefinition::new("snark-work-top-producers");

/// CF for sorting all snark work fee totals
pub(crate) const SNARK_TOP_PRODUCERS_SORT: TableDefinition<u64, PublicKey> =
    TableDefinition::new("snark-work-top-producers-sort");

/// CF for storing/sorting SNARK work fees
pub(crate) const SNARK_WORK_FEES: TableDefinition<(u64, u32, PublicKey, BlockHash), u32> =
    TableDefinition::new("snark-work-fees");

/// CF for storing/sorting SNARKs by prover
/// `{prover}{slot}{index} -> snark`
/// - prover: 55 pk bytes
/// - slot:   4 BE bytes
/// - index:  4 BE bytes
pub(crate) const SNARK_WORK_PROVER: TableDefinition<(PublicKey, u32, u32), SnarkWithCanonicity> =
    TableDefinition::new("snark-work-prover");

////////////////////////
// Username store CFs //
////////////////////////

/// CF for storing usernames
pub(crate) const USERNAME: TableDefinition<PublicKey, String> = TableDefinition::new("usernames");

pub(crate) const USERNAME_PK_NUM: TableDefinition<BlockHash, u32> =
    TableDefinition::new("username-pk-num");

pub(crate) const USERNAME_PK_INDEX: TableDefinition<Vec<u8>, Username> =
    TableDefinition::new("username-pk-index");

/// CF for storing state hash -> usernames
pub(crate) const USERNAMES_PER_BLOCK: TableDefinition<BlockHash, UsernameUpdate> =
    TableDefinition::new("usernames-per-block");

/////////////////////
// Chain store CFs //
/////////////////////

/// CF for storing chain_id -> network
pub(crate) const CHAIN_ID_TO_NETWORK: TableDefinition<ChainId, Network> =
    TableDefinition::new("chain-id-to-network");

/////////////////////
// Event store CFs //
/////////////////////

pub(crate) const EVENTS: TableDefinition<u32, IndexerEvent> = TableDefinition::new("events");

////////////////////
// Data count CFs //
////////////////////

/// CF for per epoch per account block prodution info
/// - key: `{epoch BE bytes}{pk}`
/// - value: number of blocks produced by `pk` in `epoch`
pub(crate) const BLOCK_PRODUCTION_PK_EPOCH: TableDefinition<(u32, PublicKey), u32> =
    TableDefinition::new("block-production-pk-epoch");

/// CF for per account total block prodution info
/// - key: `pk`
/// - value: total number of blocks produced by `pk`
pub(crate) const BLOCK_PRODUCTION_PK_TOTAL: TableDefinition<PublicKey, u32> =
    TableDefinition::new("block-production-pk-total");

/// CF for per epoch block production totals
/// - key: `epoch`
/// - value: number of blocks produced in `epoch`
pub(crate) const BLOCK_PRODUCTION_EPOCH: TableDefinition<u32, u32> =
    TableDefinition::new("block-production-epoch");

/// CF for per block SNARK counts
/// - key: state hash
/// - value: number of SNARKs in block
pub(crate) const BLOCK_SNARK_COUNTS: TableDefinition<BlockHash, u32> =
    TableDefinition::new("block-snark-counts");

/// CF for per block user command counts
/// - key: state hash
/// - value: number of user commands in block
pub(crate) const BLOCK_USER_COMMAND_COUNTS: TableDefinition<BlockHash, u32> =
    TableDefinition::new("block-user-command-counts");

/// CF for per block internal command counts
/// - key: state hash
/// - value: number of internal commands in block
pub(crate) const BLOCK_INTERNAL_COMMAND_COUNTS: TableDefinition<BlockHash, u32> =
    TableDefinition::new("block-internal-command-counts");

/// CF for per epoch per account user commands
/// - key: `{epoch BE bytes}{pk}`
/// - value: number of `pk` user commands in `epoch`
pub(crate) const USER_COMMANDS_PK_EPOCH: TableDefinition<(u32, PublicKey), u32> =
    TableDefinition::new("user-commands-pk-epoch");

/// CF for per account total user commands
/// - key: `pk`
/// - value: total number of `pk` user commands
pub(crate) const USER_COMMANDS_PK_TOTAL: TableDefinition<PublicKey, u32> =
    TableDefinition::new("user-commands-pk-total");

/// CF for per epoch total user commands
/// - key: `epoch`
/// - value: number of user commands in `epoch`
pub(crate) const USER_COMMANDS_EPOCH: TableDefinition<u32, u32> =
    TableDefinition::new("user-commands-epoch");

/// CF for per epoch per account internal commands
/// - key: `{epoch BE bytes}{pk}`
/// - value: number of `pk` internal commands in `epoch`
pub(crate) const INTERNAL_COMMANDS_PK_EPOCH: TableDefinition<(u32, PublicKey), u32> =
    TableDefinition::new("internal-commands-pk-epoch");

/// CF for per account total internal commands
/// - key: `pk`
/// - value: total number of `pk` internal commands
pub(crate) const INTERNAL_COMMANDS_PK_TOTAL: TableDefinition<PublicKey, u32> =
    TableDefinition::new("internal-commands-pk-total");

/// CF for per epoch total internal commands
/// - key: `epoch`
/// - value: number of internal commands in `epoch`
pub(crate) const INTERNAL_COMMANDS_EPOCH: TableDefinition<u32, u32> =
    TableDefinition::new("internal-commands-epoch");

/// CF for per epoch per account SNARKs
/// - key: `{epoch BE bytes}{pk}`
/// - value: number of `pk` SNARKs in `epoch`
pub(crate) const SNARKS_PK_EPOCH: TableDefinition<(u32, PublicKey), u32> =
    TableDefinition::new("snarks-pk-epoch");

/// CF for per account total SNARKs
/// - key: `pk`
/// - value: total number of `pk` SNARKs
pub(crate) const SNARKS_PK_TOTAL: TableDefinition<PublicKey, u32> =
    TableDefinition::new("snarks-pk-total");

/// CF for per epoch total SNARKs
/// - key: `epoch`
/// - value: number of SNARKs in `epoch`
pub(crate) const SNARKS_EPOCH: TableDefinition<u32, u32> = TableDefinition::new("snarks-epoch");

/// CF for storing u32 values
pub(crate) const INDEXED_U32: TableDefinition<&str, u32> = TableDefinition::new("indexed-u32");

/// CF for storing misc string keys
pub(crate) const STRING_KEYS: TableDefinition<&str, String> = TableDefinition::new("string-keys");

/// CF for storing IndexerStoreVersion
pub(crate) const INDEXER_STORE_VERSION: TableDefinition<&str, IndexerStoreVersion> =
    TableDefinition::new("indexer-store-version");
