use super::{
    database::{
        BLOCKS_AT_HEIGHT, BLOCKS_GLOBAL_SLOT_SORT, BLOCKS_HEIGHT_SORT, BLOCK_COINBASE_HEIGHT_SORT,
        BLOCK_COINBASE_SLOT_SORT, BLOCK_CREATOR_HEIGHT_SORT, BLOCK_CREATOR_SLOT_SORT,
    },
    fixed_keys::FixedKeys,
    DBIterator, Direction, IteratorAnchor,
};
use crate::{
    block::{
        precomputed::{PcbVersion, PrecomputedBlock},
        store::BlockStore,
        BlockComparison, BlockHash,
    },
    canonicity::{store::CanonicityStore, Canonicity},
    command::{internal::store::InternalCommandStore, store::UserCommandStore},
    event::{db::*, store::EventStore, IndexerEvent},
    ledger::{diff::LedgerDiff, public_key::PublicKey, store::LedgerStore},
    snark_work::store::SnarkStore,
    store::{
        account::{AccountBalanceUpdate, AccountStore},
        block_state_hash_from_key, block_u32_prefix_from_key, from_be_bytes, to_be_bytes,
        u32_prefix_key,
        username::UsernameStore,
        DBUpdate, IndexerStore,
    },
};
use anyhow::bail;
use log::{error, trace};

impl BlockStore for IndexerStore {
    /// Add the given block at its indices and record a db event
    fn add_block(&self, block: &PrecomputedBlock) -> anyhow::Result<Option<DbEvent>> {
        trace!("Adding block {}", block.summary());

        // add block to db
        let state_hash = block.state_hash();

        if matches!(self.get_block(&state_hash), Ok(Some(_))) {
            trace!("Block already present {}", block.summary());
            return Ok(None);
        }
        self.put(BLOCKS, &state_hash, block);

        // add to ledger diff index
        self.set_block_ledger_diff(&state_hash, LedgerDiff::from_precomputed(block))?;

        // add to epoch index before setting other indices
        self.set_block_epoch(&state_hash, block.epoch_count())?;

        // increment block production counts
        self.increment_block_production_count(block)?;

        // add comparison data before user commands, SNARKs, and internal commands
        self.set_block_comparison(&state_hash, &BlockComparison::from(block))?;

        // add to blockchain length index
        self.set_block_height(&state_hash, block.blockchain_length())?;

        // add to block global slot index
        self.set_block_global_slot(&state_hash, block.global_slot_since_genesis())?;

        // add to parent hash index
        self.set_block_parent_hash(&state_hash, &block.previous_state_hash())?;

        // add to staged ledger hash index
        self.set_block_staged_ledger_hash(&state_hash, &block.staged_ledger_hash())?;

        // add to genesis state hash index
        self.set_block_genesis_state_hash(&state_hash, &block.genesis_state_hash())?;

        // add block height/global slot index
        self.set_block_height_global_slot_pair(
            block.blockchain_length(),
            block.global_slot_since_genesis(),
        )?;

        // add to block creator index
        self.set_block_creator(block)?;

        // add to coinbase receiver index
        self.set_coinbase_receiver(block)?;

        // add to balance update index
        self.set_block_balance_updates(
            &state_hash,
            &<DBUpdate<AccountBalanceUpdate>>::from_precomputed(block),
        )?;

        // add block height/global slot for sorting
        self.put_sort(BLOCKS_HEIGHT_SORT, block_height_key(block))?;
        self.put_sort(BLOCKS_GLOBAL_SLOT_SORT, block_global_slot_key(block))?;

        // add block for each public key
        for pk in block.all_public_keys() {
            self.add_block_at_public_key(&pk, &state_hash)?;
        }

        // add block to height list
        self.add_block_at_height(&state_hash, block.blockchain_length())?;

        // add block to slots list
        self.add_block_at_slot(&state_hash, block.global_slot_since_genesis())?;

        // add pcb's version
        self.set_block_version(&state_hash, block.version())?;

        // add block user commands
        self.add_user_commands(block)?;

        // add block internal commands
        self.add_internal_commands(block)?;

        // add block SNARK work
        self.add_snark_work(block)?;

        // add new block db event only after all other data is added
        let db_event = DbEvent::Block(DbBlockEvent::NewBlock {
            state_hash: block.state_hash(),
            blockchain_length: block.blockchain_length(),
        });
        self.add_event(&IndexerEvent::Db(db_event.clone()))?;

        Ok(Some(db_event))
    }

    fn get_block(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PrecomputedBlock>> {
        trace!("Getting block {state_hash}");
        Ok(self.get(BLOCKS, state_hash))
    }

    fn get_best_block(&self) -> anyhow::Result<Option<PrecomputedBlock>> {
        trace!("Getting best block");
        match self.get_best_block_hash()? {
            None => Ok(None),
            Some(state_hash) => self.get_block(&state_hash),
        }
    }

    fn get_best_block_hash(&self) -> anyhow::Result<Option<BlockHash>> {
        trace!("Getting best block hash");

        let block_hash: Option<BlockHash> = self.get(STRING_KEYS, Self::BEST_TIP_STATE_HASH_KEY);
        Ok(block_hash)
    }

    fn get_best_block_height(&self) -> anyhow::Result<Option<u32>> {
        Ok(self
            .get_best_block_hash()?
            .and_then(|state_hash| self.get_block_height(&state_hash).ok().flatten()))
    }

    fn get_best_block_global_slot(&self) -> anyhow::Result<Option<u32>> {
        Ok(self
            .get_best_block_hash()?
            .and_then(|state_hash| self.get_block_global_slot(&state_hash).ok().flatten()))
    }

    fn get_best_block_genesis_hash(&self) -> anyhow::Result<Option<BlockHash>> {
        Ok(self.get_best_block_hash()?.and_then(|state_hash| {
            self.get_block_genesis_state_hash(&state_hash)
                .ok()
                .flatten()
        }))
    }

    fn set_best_block(&self, state_hash: &BlockHash) -> anyhow::Result<()> {
        trace!("Setting best block {state_hash}");

        if let Some(old) = self.get_best_block_hash()? {
            if old == *state_hash {
                return Ok(());
            }

            // reorg updates
            // canonicity
            let canonicity_updates = self.reorg_canonicity_updates(&old, state_hash)?;
            self.update_canonicity(canonicity_updates)?;

            // balance-sorted accounts
            let balance_updates = self.reorg_account_balance_updates(&old, state_hash)?;
            self.update_account_balances(state_hash, &balance_updates)?;

            // usernames
            let username_updates = self.reorg_username_updates(&old, state_hash)?;
            self.update_usernames(username_updates)?;
        }

        // set new best tip
        self.database
            .write(STRING_KEYS, Self::BEST_TIP_STATE_HASH_KEY, state_hash.0);

        // record new best tip event
        match self.get_block_height(state_hash)? {
            Some(blockchain_length) => {
                self.add_event(&IndexerEvent::Db(DbEvent::Block(
                    DbBlockEvent::NewBestTip {
                        state_hash: state_hash.clone(),
                        blockchain_length,
                    },
                )))?;
            }
            None => error!("Block missing from store: {state_hash}"),
        }
        Ok(())
    }

    fn get_block_parent_hash(&self, state_hash: &BlockHash) -> anyhow::Result<Option<BlockHash>> {
        trace!("Getting block's parent hash {state_hash}");
        Ok(self.get(BLOCK_PARENT_HASH, state_hash))
    }

    fn set_block_parent_hash(
        &self,
        state_hash: &BlockHash,
        previous_state_hash: &BlockHash,
    ) -> anyhow::Result<()> {
        trace!(
            "Setting block parent hash {} -> {}",
            state_hash,
            previous_state_hash
        );

        Ok(self.put(BLOCK_PARENT_HASH, state_hash, previous_state_hash))
    }

    fn get_block_height(&self, state_hash: &BlockHash) -> anyhow::Result<Option<u32>> {
        trace!("Getting blockchain length {state_hash}");
        Ok(self.get(BLOCK_HEIGHT, state_hash))
    }

    fn set_block_height(
        &self,
        state_hash: &BlockHash,
        blockchain_length: u32,
    ) -> anyhow::Result<()> {
        trace!("Setting blockchain length {blockchain_length}: {state_hash}");
        Ok(self.put(BLOCK_HEIGHT, state_hash, blockchain_length))
    }

    fn get_block_global_slot(&self, state_hash: &BlockHash) -> anyhow::Result<Option<u32>> {
        trace!("Getting block global slot {state_hash}");
        Ok(self.get(BLOCK_GLOBAL_SLOT, state_hash))
    }

    fn set_block_global_slot(
        &self,
        state_hash: &BlockHash,
        global_slot: u32,
    ) -> anyhow::Result<()> {
        trace!("Setting block global slot {state_hash}: {global_slot}");
        Ok(self.put(BLOCK_GLOBAL_SLOT, state_hash, global_slot)?)
    }

    fn get_block_creator(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PublicKey>> {
        trace!("Getting block creator {state_hash}");
        Ok(self.get(BLOCK_CREATOR, state_hash))
    }

    fn set_block_creator(&self, block: &PrecomputedBlock) -> anyhow::Result<()> {
        let state_hash = block.state_hash();
        let block_creator = block.block_creator();
        trace!("Setting block creator: {state_hash} -> {block_creator}");

        // index
        self.put(BLOCK_CREATOR, state_hash, block_creator)?;

        // block height sort
        self.put_sort(
            BLOCK_CREATOR_HEIGHT_SORT,
            pk_block_sort_key(
                block_creator.clone(),
                block.blockchain_length(),
                state_hash.clone(),
            ),
        )?;

        // global slot sort
        Ok(self.put_sort(
            BLOCK_CREATOR_SLOT_SORT,
            pk_block_sort_key(block_creator, block.global_slot_since_genesis(), state_hash),
        )?)
    }

    fn get_coinbase_receiver(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PublicKey>> {
        trace!("Getting coinbase receiver for {state_hash}");
        Ok(self.get(BLOCK_COINBASE_RECEIVER, state_hash))
    }

    fn set_coinbase_receiver(&self, block: &PrecomputedBlock) -> anyhow::Result<()> {
        let state_hash = block.state_hash();
        let coinbase_receiver = block.coinbase_receiver();
        trace!("Setting coinbase receiver: {state_hash} -> {coinbase_receiver}");
        Ok(self.put(BLOCK_COINBASE_RECEIVER, state_hash, coinbase_receiver))
    }

    fn get_num_blocks_at_height(&self, blockchain_length: u32) -> anyhow::Result<u32> {
        trace!("Getting number of blocks at height {blockchain_length}");
        Ok(self
            .get(BLOCKS_AT_HEIGHT, to_be_bytes(blockchain_length))?
            .map_or(0, from_be_bytes))
    }

    fn add_block_at_height(
        &self,
        state_hash: &BlockHash,
        blockchain_length: u32,
    ) -> anyhow::Result<()> {
        trace!("Adding block {state_hash} at height {blockchain_length}");

        // increment num blocks at height
        let num_blocks_at_height = self.get_num_blocks_at_height(blockchain_length)?;
        self.put(
            BLOCKS_AT_HEIGHT,
            to_be_bytes(blockchain_length),
            to_be_bytes(num_blocks_at_height + 1),
        )?;

        // add the new key-value pair
        Ok(self.put(
            BLOCKS_AT_HEIGHT,
            format!("{blockchain_length}-{num_blocks_at_height}"),
            state_hash.0.as_bytes(),
        )?)
    }

    fn get_blocks_at_height(&self, blockchain_length: u32) -> anyhow::Result<Vec<BlockHash>> {
        let num_blocks_at_height = self.get_num_blocks_at_height(blockchain_length)?;
        let mut blocks = vec![];

        for n in 0..num_blocks_at_height {
            match self.get(BLOCKS_AT_HEIGHT, format!("{blockchain_length}-{n}"))? {
                None => break,
                Some(bytes) => blocks.push(BlockHash::from_bytes(&bytes)?),
            }
        }
        blocks.sort_by(|a, b| block_cmp(self, a, b));
        Ok(blocks)
    }

    fn get_num_blocks_at_slot(&self, slot: u32) -> anyhow::Result<u32> {
        trace!("Getting number of blocks at slot {slot}");
        Ok(self
            .get(BLOCKS_AT_GLOBAL_SLOT, to_be_bytes(slot))?
            .map_or(0, from_be_bytes))
    }

    fn add_block_at_slot(&self, state_hash: &BlockHash, slot: u32) -> anyhow::Result<()> {
        trace!("Adding block {state_hash} at slot {slot}");

        // increment num blocks at slot
        let num_blocks_at_slot = self.get_num_blocks_at_slot(slot)?;
        self.put(
            BLOCKS_AT_GLOBAL_SLOT,
            to_be_bytes(slot),
            to_be_bytes(num_blocks_at_slot + 1),
        )?;

        // add the new key-value pair
        Ok(self.put(
            BLOCKS_AT_GLOBAL_SLOT,
            format!("{slot}-{num_blocks_at_slot}"),
            state_hash.0.as_bytes(),
        )?)
    }

    fn get_blocks_at_slot(&self, slot: u32) -> anyhow::Result<Vec<BlockHash>> {
        trace!("Getting blocks at slot {slot}");

        let num_blocks_at_slot = self.get_num_blocks_at_slot(slot)?;
        let mut blocks = vec![];

        for n in 0..num_blocks_at_slot {
            match self.get(BLOCKS_AT_GLOBAL_SLOT, format!("{slot}-{n}"))? {
                None => break,
                Some(bytes) => blocks.push(BlockHash::from_bytes(&bytes)?),
            }
        }
        blocks.sort_by(|a, b| block_cmp(self, a, b));
        Ok(blocks)
    }

    fn get_num_blocks_at_public_key(&self, pk: &PublicKey) -> anyhow::Result<u32> {
        trace!("Getting number of blocks at public key {pk}");
        Ok(
            match self
                .database
                .get_pinned_cf(self.blocks_cf(), pk.to_string().as_bytes())?
            {
                None => 0,
                Some(bytes) => String::from_utf8(bytes.to_vec())?.parse()?,
            },
        )
    }

    fn add_block_at_public_key(
        &self,
        pk: &PublicKey,
        state_hash: &BlockHash,
    ) -> anyhow::Result<()> {
        trace!("Adding block {state_hash} at public key {pk}");

        // increment num blocks at public key
        let num_blocks_at_pk = self.get_num_blocks_at_public_key(pk)?;
        self.put(
            self.blocks_cf(),
            pk.to_string().as_bytes(),
            (num_blocks_at_pk + 1).to_string().as_bytes(),
        )?;

        // add the new key-value pair
        let key = format!("{pk}-{num_blocks_at_pk}");
        Ok(self.put(
            self.blocks_cf(),
            key.as_bytes(),
            state_hash.to_string().as_bytes(),
        )?)
    }

    fn get_blocks_at_public_key(&self, pk: &PublicKey) -> anyhow::Result<Vec<BlockHash>> {
        trace!("Getting blocks at public key {pk}");

        let num_blocks_at_pk = self.get_num_blocks_at_public_key(pk)?;
        let mut blocks = vec![];

        for n in 0..num_blocks_at_pk {
            let key = format!("{pk}-{n}");
            match self
                .database
                .get_pinned_cf(self.blocks_cf(), key.as_bytes())?
            {
                None => break,
                Some(bytes) => blocks.push(BlockHash::from_bytes(&bytes)?),
            }
        }
        blocks.sort_by(|a, b| block_cmp(self, a, b));
        Ok(blocks)
    }

    fn get_block_children(&self, state_hash: &BlockHash) -> anyhow::Result<Vec<PrecomputedBlock>> {
        trace!("Getting children of block {state_hash}");

        if let Some(height) = self.get_block(state_hash)?.map(|b| b.blockchain_length()) {
            let blocks_at_next_height = self.get_blocks_at_height(height + 1)?;
            let mut children: Vec<BlockHash> = blocks_at_next_height
                .into_iter()
                .filter(|b| {
                    self.get_block_parent_hash(b).ok().flatten() == Some(state_hash.clone())
                })
                .collect();
            children.sort_by(|a, b| block_cmp(self, a, b));
            return Ok(children);
        }
        bail!("Block missing from store {state_hash}")
    }

    fn get_block_version(&self, state_hash: &BlockHash) -> anyhow::Result<Option<PcbVersion>> {
        trace!("Getting block version {state_hash}");
        Ok(self.get(BLOCKS_VERSION, state_hash))
    }

    fn set_block_version(&self, state_hash: &BlockHash, version: PcbVersion) -> anyhow::Result<()> {
        trace!("Setting block {state_hash} version to {version}");
        Ok(self.put(BLOCKS_VERSION, state_hash, version))
    }

    fn set_block_height_global_slot_pair(
        &self,
        blockchain_length: u32,
        global_slot: u32,
    ) -> anyhow::Result<()> {
        trace!("Setting block height {blockchain_length} <-> slot {global_slot}");

        // add height to slot's "height collection"
        let mut heights = self
            .get_block_heights_from_global_slot(global_slot)?
            .unwrap_or_default();
        if !heights.contains(&blockchain_length) {
            heights.push(blockchain_length);
            heights.sort();
            self.put(
                self.block_global_slot_to_heights_cf(),
                to_be_bytes(global_slot),
                serde_json::to_vec(&heights)?,
            )?;
        }

        // add slot to height's "slot collection"
        let mut slots = self
            .get_block_global_slots_from_height(blockchain_length)?
            .unwrap_or_default();
        if !slots.contains(&global_slot) {
            slots.push(global_slot);
            slots.sort();
            self.put(
                self.block_height_to_global_slots_cf(),
                to_be_bytes(blockchain_length),
                serde_json::to_vec(&slots)?,
            )?;
        }
        Ok(())
    }

    fn get_block_global_slots_from_height(
        &self,
        blockchain_length: u32,
    ) -> anyhow::Result<Option<Vec<u32>>> {
        trace!("Getting global slot for height {blockchain_length}");
        Ok(self.get(BLOCK_GLOBAL_SLOT_TO_HEIGHT, blockchain_length))
    }

    fn get_block_heights_from_global_slot(
        &self,
        global_slot_since_genesis: u32,
    ) -> anyhow::Result<Option<Vec<u32>>> {
        trace!("Getting height for global slot {global_slot}");
        Ok(self.get(BLOCK_HEIGHT_TO_GLOBAL_SLOTS, global_slot_since_genesis))
    }

    fn get_current_epoch(&self) -> anyhow::Result<u32> {
        Ok(self
            .get_best_block_hash()?
            .and_then(|state_hash| self.get_block_epoch(&state_hash).ok().flatten())
            .unwrap_or_default())
    }

    fn set_block_epoch(&self, state_hash: &BlockHash, epoch: u32) -> anyhow::Result<()> {
        trace!("Setting block epoch {epoch}: {state_hash}");
        Ok(self.put(BLOCK_EPOCH, state_hash, epoch)?)
    }

    fn get_block_epoch(&self, state_hash: &BlockHash) -> anyhow::Result<Option<u32>> {
        trace!("Getting block epoch {state_hash}");
        Ok(self.get(BLOCK_EPOCH, state_hash))
    }

    fn set_block_genesis_state_hash(
        &self,
        state_hash: &BlockHash,
        genesis_state_hash: &BlockHash,
    ) -> anyhow::Result<()> {
        trace!("Setting block genesis state hash {state_hash}: {genesis_state_hash}");
        Ok(self.put(BLOCK_GENESIS_STATE_HASH, state_hash, genesis_state_hash)?)
    }

    fn get_block_genesis_state_hash(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<BlockHash>> {
        trace!("Getting block genesis state hash {state_hash}");
        Ok(self.get(BLOCK_GENESIS_STATE_HASH, state_hash))
    }

    ///////////////
    // Iterators //
    ///////////////

    fn blocks_height_iterator<'a>(
        &'a self,
        anchor: IteratorAnchor,
    ) -> DBIterator<Vec<u8>, &[u8; 0]> {
        self.iterator(BLOCKS_HEIGHT_SORT, anchor)
    }

    fn blocks_global_slot_iterator<'a>(
        &'a self,
        anchor: IteratorAnchor,
    ) -> DBIterator<Vec<u8>, &[u8; 0]> {
        self.iterator(BLOCKS_GLOBAL_SLOT_SORT, anchor)
    }

    fn block_creator_block_height_iterator<'a>(
        &'a self,
        anchor: IteratorAnchor,
    ) -> DBIterator<Vec<u8>, &[u8; 0]> {
        self.iterator(BLOCK_CREATOR_HEIGHT_SORT, anchor)
    }

    fn block_creator_global_slot_iterator<'a>(
        &'a self,
        anchor: IteratorAnchor,
    ) -> DBIterator<Vec<u8>, &[u8; 0]> {
        self.iterator(BLOCK_CREATOR_SLOT_SORT, anchor)
    }

    fn coinbase_receiver_block_height_iterator<'a>(
        &'a self,
        anchor: IteratorAnchor,
    ) -> DBIterator<Vec<u8>, &[u8; 0]> {
        self.iterator(BLOCK_COINBASE_HEIGHT_SORT, anchor)
    }

    fn coinbase_receiver_global_slot_iterator<'a>(
        &'a self,
        anchor: IteratorAnchor,
    ) -> DBIterator<Vec<u8>, &[u8; 0]> {
        self.iterator(BLOCK_COINBASE_SLOT_SORT, anchor)
    }

    //////////////////
    // Block counts //
    //////////////////

    fn increment_block_production_count(&self, block: &PrecomputedBlock) -> anyhow::Result<()> {
        trace!("Incrementing block production count {}", block.summary());

        let creator = block.block_creator();
        let epoch = block.epoch_count();

        // increment pk epoch count
        let acc = self.get_block_production_pk_epoch_count(&creator, Some(epoch))?;
        self.put(BLOCK_PRODUCTION_PK_EPOCH, (epoch, &creator), acc + 1);

        // increment pk total count
        let acc = self.get_block_production_pk_total_count(&creator)?;
        self.put(BLOCK_PRODUCTION_PK_TOTAL, creator, acc + 1);

        // increment epoch count
        let acc = self.get_block_production_epoch_count(Some(epoch))?;
        self.put(BLOCK_PRODUCTION_EPOCH, epoch, acc + 1);

        // increment total count
        let acc = self.get_block_production_total_count()?;
        self.database
            .put(Self::TOTAL_NUM_BLOCKS_KEY, to_be_bytes(acc + 1))?;

        Ok(())
    }

    fn get_block_production_pk_epoch_count(
        &self,
        pk: &PublicKey,
        epoch: Option<u32>,
    ) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting pk epoch {epoch} block production count {pk}");
        Ok(self.get(BLOCK_PRODUCTION_PK_EPOCH, (epoch, pk)))
    }

    fn get_block_production_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<u32> {
        trace!("Getting pk total block production count {pk}");
        Ok(self.get(BLOCK_PRODUCTION_PK_TOTAL, pk))
    }

    fn get_block_production_epoch_count(&self, epoch: Option<u32>) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting epoch block production count {epoch}");
        Ok(self.get(BLOCK_PRODUCTION_EPOCH, epoch))
    }

    fn get_block_production_total_count(&self) -> anyhow::Result<u32> {
        trace!("Getting total block production count");
        Ok(self
            .database
            .get(Self::TOTAL_NUM_BLOCKS_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn set_block_comparison(
        &self,
        state_hash: &BlockHash,
        comparison: &BlockComparison,
    ) -> anyhow::Result<()> {
        trace!("Setting block comparison {state_hash}");
        Ok(self.put(BLOCK_COMPARISON, state_hash.0, comparison?)?)
    }

    fn get_block_comparison(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<BlockComparison>> {
        trace!("Getting block comparison {state_hash}");
        Ok(self.get(BLOCK_COMPARISON, state_hash))
    }

    fn block_cmp(
        &self,
        block: &BlockHash,
        other: &BlockHash,
    ) -> anyhow::Result<Option<std::cmp::Ordering>> {
        // get stored block comparisons
        let res1 = self.get(BLOCK_COMPARISON, block);
        let res2 = self.get(BLOCK_COMPARISON, other);

        // compare stored block comparisons
        if let (Some(bc1), Some(bc2)) = (res1, res2) {
            return Ok(Some(bc1.cmp(&bc2)));
        }
        Ok(None)
    }

    fn dump_blocks_via_height(&self, path: &std::path::Path) -> anyhow::Result<()> {
        use std::{fs::File, io::Write};
        trace!("Dumping blocks via height to {}", path.display());
        let mut file = File::create(path)?;

        for (key, _) in self.blocks_height_iterator(IteratorAnchor::Start).flatten() {
            let state_hash = block_state_hash_from_key(&key)?;
            let block_height = block_u32_prefix_from_key(&key)?;
            let global_slot = self
                .get_block_global_slot(&state_hash)?
                .expect("global slot");

            writeln!(
                file,
                "height: {block_height}\nslot:   {global_slot}\nstate:  {state_hash}"
            )?;
        }
        Ok(())
    }

    fn blocks_via_height(&self, anchor: IteratorAnchor) -> anyhow::Result<Vec<PrecomputedBlock>> {
        let mut blocks = vec![];
        trace!("Getting blocks via height (mode: {})", display_mode(anchor));
        for (key, _) in self.blocks_height_iterator(anchor).flatten() {
            let state_hash = block_state_hash_from_key(&key)?;
            blocks.push(self.get_block(&state_hash)?.expect("PCB"));
        }
        Ok(blocks)
    }

    fn dump_blocks_via_global_slot(&self, path: &std::path::Path) -> anyhow::Result<()> {
        use std::{fs::File, io::Write};
        trace!("Dumping blocks via global slot to {}", path.display());
        let mut file = File::create(path)?;

        for (key, _) in self
            .blocks_global_slot_iterator(IteratorAnchor::Start)
            .flatten()
        {
            let state_hash = block_state_hash_from_key(&key)?;
            let block_height = block_u32_prefix_from_key(&key)?;
            let global_slot = self
                .get_block_global_slot(&state_hash)?
                .expect("global slot");

            writeln!(
                file,
                "height: {block_height}\nslot:   {global_slot}\nstate:  {state_hash}"
            )?;
        }
        Ok(())
    }

    fn blocks_via_global_slot(
        &self,
        anchor: IteratorAnchor,
    ) -> anyhow::Result<Vec<PrecomputedBlock>> {
        let mut blocks = vec![];
        trace!(
            "Getting blocks via global slot (mode: {})",
            display_mode(anchor)
        );
        for (key, _) in self.blocks_global_slot_iterator(anchor).flatten() {
            let state_hash = block_state_hash_from_key(&key)?;
            blocks.push(self.get_block(&state_hash)?.expect("PCB"));
        }
        Ok(blocks)
    }
}

/// `{block height BE}{state hash}`
fn block_height_key(block: &PrecomputedBlock) -> Vec<u8> {
    let mut key = to_be_bytes(block.blockchain_length());
    key.append(&mut block.state_hash().to_bytes());
    key
}

/// `{global slot BE}{state hash}`
fn block_global_slot_key(block: &PrecomputedBlock) -> Vec<u8> {
    let mut key = to_be_bytes(block.global_slot_since_genesis());
    key.append(&mut block.state_hash().to_bytes());
    key
}

/// `{pk}{height/slot BE}{state hash}`
fn pk_block_sort_key(pk: PublicKey, sort_value: u32, state_hash: BlockHash) -> Vec<u8> {
    let mut key = pk.to_bytes();
    key.append(&mut to_be_bytes(sort_value));
    key.append(&mut state_hash.to_bytes());
    key
}

fn block_cmp(db: &IndexerStore, a: &BlockHash, b: &BlockHash) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let a_canonicity = db.get_block_canonicity(a).ok().flatten();
    let b_canonicity = db.get_block_canonicity(b).ok().flatten();
    let a_cmp = db.get_block_comparison(a).unwrap().unwrap();
    let b_cmp = db.get_block_comparison(b).unwrap().unwrap();
    match (a_canonicity, b_canonicity) {
        (Some(Canonicity::Canonical), _) => Ordering::Less,
        (_, Some(Canonicity::Canonical)) => Ordering::Greater,
        _ => a_cmp.cmp(&b_cmp),
    }
}

fn display_mode(mode: IteratorMode) -> String {
    match mode {
        IteratorMode::End => "End".to_string(),
        IteratorMode::Start => "Start".to_string(),
        IteratorMode::From(start, direction) => {
            format!("{} from {start:?}", display_direction(direction))
        }
    }
}

fn display_direction(direction: Direction) -> String {
    match direction {
        Direction::Forward => "Forward".to_string(),
        Direction::Reverse => "Reverse".to_string(),
    }
}
