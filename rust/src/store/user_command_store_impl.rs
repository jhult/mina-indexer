use super::{
    database::{
        TXN_FROM_HEIGHT_SORT, TXN_TO_HEIGHT_SORT, USER_COMMANDS_HEIGHT_SORT,
        USER_COMMANDS_SLOT_SORT,
    },
    fixed_keys::FixedKeys,
    DBIterator, IteratorAnchor,
};
use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore, BlockComparison, BlockHash},
    command::{
        signed::{SignedCommand, SignedCommandWithData},
        store::UserCommandStore,
        UserCommandWithStatus, UserCommandWithStatusT,
    },
    ledger::public_key::PublicKey,
    store::{
        database::{
            BLOCK_USER_COMMAND_COUNTS, USER_COMMANDS, USER_COMMANDS_EPOCH,
            USER_COMMANDS_NUM_CONTAINING_BLOCKS, USER_COMMANDS_PER_BLOCK, USER_COMMANDS_PK,
            USER_COMMANDS_PK_EPOCH, USER_COMMANDS_PK_NUM, USER_COMMANDS_PK_TOTAL,
            USER_COMMANDS_TXN_HASH_TO_GLOBAL_SLOT, USER_COMMAND_STATE_HASHES,
        },
        from_be_bytes, pk_txn_sort_key, to_be_bytes, txn_sort_key, user_command_db_key_pk,
        username::UsernameStore,
        IndexerStore,
    },
};
use log::{trace, warn};

impl UserCommandStore for IndexerStore {
    fn add_user_commands(&self, block: &PrecomputedBlock) -> anyhow::Result<()> {
        trace!("Adding user commands from block {}", block.summary());

        let epoch = block.epoch_count();
        let state_hash = block.state_hash();
        let user_commands = block.commands();

        // per block
        self.set_block_user_commands(block)?;
        self.set_block_user_commands_count(&state_hash, user_commands.len() as u32)?;
        self.set_block_username_updates(&state_hash, &block.username_updates())?;

        // per command
        for command in &user_commands {
            let signed = SignedCommand::from(command.clone());
            let txn_hash = signed.hash_signed_command()?;
            trace!("Adding user command {txn_hash} block {}", block.summary());

            // add signed command
            self.put(
                USER_COMMANDS,
                (&txn_hash, state_hash.clone()),
                &SignedCommandWithData::from(
                    command,
                    &block.state_hash().0,
                    block.blockchain_length(),
                    block.timestamp(),
                    block.global_slot_since_genesis(),
                ),
            );

            // add state hash index
            self.set_user_command_state_hash(state_hash.clone(), &txn_hash)?;

            // add index for global slot sorting
            self.put_sort(
                self.user_commands_slot_sort_cf(),
                txn_sort_key(
                    block.global_slot_since_genesis(),
                    &txn_hash,
                    state_hash.clone(),
                ),
            )?;

            // add index for block height sorting
            self.put_sort(
                self.user_commands_height_sort_cf(),
                txn_sort_key(block.blockchain_length(), &txn_hash, state_hash.clone()),
            )?;

            // increment counts
            self.increment_user_commands_counts(command, epoch)?;

            // add: `txn_hash -> global_slot`
            // so we can reconstruct the key
            // TODO: where is this used?
            self.put(
                USER_COMMANDS_TXN_HASH_TO_GLOBAL_SLOT,
                &txn_hash,
                block.global_slot_since_genesis(),
            );

            // add sender index
            self.put(
                TXN_FROM_HEIGHT_SORT,
                pk_txn_sort_key(
                    command.sender(),
                    block.blockchain_length(),
                    &txn_hash,
                    block.state_hash(),
                ),
                command.amount().to_be_bytes(),
            )?;
            self.put(
                self.txn_from_slot_sort_cf(),
                pk_txn_sort_key(
                    command.sender(),
                    block.global_slot_since_genesis(),
                    &txn_hash,
                    block.state_hash(),
                ),
                command.amount().to_be_bytes(),
            )?;

            // add receiver index
            self.put(
                self.txn_to_height_sort_cf(),
                pk_txn_sort_key(
                    command.receiver(),
                    block.blockchain_length(),
                    &txn_hash,
                    block.state_hash(),
                ),
                command.amount().to_be_bytes(),
            )?;
            self.put(
                self.txn_to_slot_sort_cf(),
                pk_txn_sort_key(
                    command.receiver(),
                    block.global_slot_since_genesis(),
                    &txn_hash,
                    block.state_hash(),
                ),
                command.amount().to_be_bytes(),
            )?;
        }

        // per account
        // add: "pk -> linked list of signed commands with state hash"
        for pk in block.all_command_public_keys() {
            let n = self
                .get_pk_num_user_commands_blocks(&pk)?
                .unwrap_or_default();
            let block_pk_commands: Vec<SignedCommandWithData> = user_commands
                .iter()
                .filter(|cmd| cmd.contains_public_key(&pk))
                .map(|c| {
                    SignedCommandWithData::from(
                        c,
                        &block.state_hash().0,
                        block.blockchain_length(),
                        block.timestamp(),
                        block.global_slot_since_genesis(),
                    )
                })
                .collect();

            if !block_pk_commands.is_empty() {
                // write these commands to the next key for pk
                self.put(
                    USER_COMMANDS_PK,
                    user_command_db_key_pk(&pk.0, n),
                    &block_pk_commands,
                )?;

                // update pk's num commands
                self.put(USER_COMMANDS_PK_NUM, pk, n + 1)?;
            }
        }
        Ok(())
    }

    fn get_user_command(
        &self,
        txn_hash: &str,
        index: u32,
    ) -> anyhow::Result<Option<SignedCommandWithData>> {
        trace!("Getting user command {txn_hash} index {index}");
        Ok(self
            .get_user_command_state_hashes(txn_hash)
            .ok()
            .flatten()
            .and_then(|b| b.get(index as usize).cloned())
            .and_then(|state_hash| {
                self.get_user_command_state_hash(txn_hash, &state_hash)
                    .unwrap()
            }))
    }

    fn get_user_command_state_hash(
        &self,
        txn_hash: &str,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<SignedCommandWithData>> {
        trace!("Getting user command {txn_hash} in block {state_hash}");
        Ok(self.get(USER_COMMANDS, (txn_hash, state_hash.clone())))
    }

    fn get_user_command_state_hashes(
        &self,
        txn_hash: &str,
    ) -> anyhow::Result<Option<Vec<BlockHash>>> {
        trace!("Getting user command blocks {txn_hash}");
        Ok(self.get(USER_COMMAND_STATE_HASHES, txn_hash))
    }

    fn set_user_command_state_hash(
        &self,
        state_hash: BlockHash,
        txn_hash: &str,
    ) -> anyhow::Result<()> {
        trace!("Setting user command {txn_hash} block {state_hash}");
        let mut blocks = self
            .get_user_command_state_hashes(txn_hash)?
            .unwrap_or_default();
        blocks.push(state_hash);

        let mut block_cmps: Vec<BlockComparison> = blocks
            .iter()
            .filter_map(|b| self.get_block_comparison(b).ok())
            .flatten()
            .collect();
        block_cmps.sort();

        let blocks: Vec<BlockHash> = block_cmps.into_iter().map(|c| c.state_hash).collect();
        // set num containing blocks
        self.put(
            USER_COMMANDS_NUM_CONTAINING_BLOCKS,
            txn_hash,
            blocks.len() as u32,
        )?;

        // set containing blocks
        self.put(USER_COMMAND_STATE_HASHES, txn_hash, &blocks)?;
        Ok(())
    }

    fn set_block_user_commands(&self, block: &PrecomputedBlock) -> anyhow::Result<()> {
        let state_hash = block.state_hash();
        trace!("Setting block user commands {state_hash}");
        Ok(self.put(USER_COMMANDS_PER_BLOCK, state_hash, &block.commands)?)
    }

    fn get_block_user_commands(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<Vec<UserCommandWithStatus>>> {
        trace!("Getting block user commands {state_hash}");
        Ok(self.get(USER_COMMANDS_PER_BLOCK, state_hash))
    }

    fn get_user_commands_for_public_key(
        &self,
        pk: &PublicKey,
    ) -> anyhow::Result<Option<Vec<SignedCommandWithData>>> {
        trace!("Getting user commands for public key {pk}");

        let mut commands = vec![];

        if let Some(n) = self.get_pk_num_user_commands_blocks(pk)? {
            // collect user commands from all pk's blocks
            for m in 0..n {
                if let Some(mut block_m_commands) =
                    self.get(USER_COMMANDS_PK, user_command_db_key_pk(pk, m))
                {
                    commands.append(&mut block_m_commands);
                } else {
                    commands.clear();
                    break;
                }
            }
            return Ok(Some(commands));
        }
        Ok(None)
    }

    fn get_user_commands_with_bounds(
        &self,
        pk: &PublicKey,
        start_state_hash: &BlockHash,
        end_state_hash: &BlockHash,
    ) -> anyhow::Result<Vec<SignedCommandWithData>> {
        let start_block_opt = self.get_block(start_state_hash)?;
        let end_block_opt = self.get_block(end_state_hash)?;
        trace!(
            "Getting user commands between {:?} and {:?}",
            start_block_opt.as_ref().map(|b| b.summary()),
            end_block_opt.as_ref().map(|b| b.summary())
        );

        if let (Some(start_block), Some(end_block)) = (start_block_opt, end_block_opt) {
            let start_height = start_block.blockchain_length();
            let end_height = end_block.blockchain_length();

            if end_height < start_height {
                warn!("Block (length {end_height}) {end_state_hash} is lower than block (length {start_height}) {start_state_hash}");
                return Ok(vec![]);
            }

            let mut num = end_height - start_height;
            let mut prev_hash = end_block.previous_state_hash();
            let mut state_hashes: Vec<BlockHash> = vec![end_block.state_hash()];
            while let Some(block) = self.get_block(&prev_hash)? {
                if num == 0 {
                    break;
                }

                num -= 1;
                state_hashes.push(prev_hash);
                prev_hash = block.previous_state_hash();
            }

            if let Ok(Some(cmds)) = self.get_user_commands_for_public_key(pk) {
                return Ok(cmds
                    .into_iter()
                    .filter(|c| state_hashes.contains(&c.state_hash))
                    .collect());
            }
        }
        Ok(vec![])
    }

    fn get_user_commands_num_containing_blocks(
        &self,
        txn_hash: &str,
    ) -> anyhow::Result<Option<u32>> {
        trace!("Getting user commands num containing blocks {txn_hash}");
        Ok(self
            .get(USER_COMMANDS_NUM_CONTAINING_BLOCKS, txn_hash)?
            .map(from_be_bytes))
    }

    ///////////////
    // Iterators //
    ///////////////

    fn user_commands_slot_iterator<'a>(&'a self, anchor: IteratorAnchor) -> DBIterator<K, V> {
        self.iterator(USER_COMMANDS_SLOT_SORT, anchor)
    }

    fn user_commands_height_iterator<'a>(&'a self, anchor: IteratorAnchor) -> DBIterator<K, V> {
        self.iterator(USER_COMMANDS_HEIGHT_SORT, anchor)
    }

    fn txn_from_height_iterator<'a>(&'a self, anchor: IteratorAnchor) -> DBIterator<Vec<u8>, u32> {
        self.iterator(TXN_FROM_HEIGHT_SORT, anchor)
    }

    fn txn_to_height_iterator<'a>(&'a self, anchor: IteratorAnchor) -> DBIterator<K, V> {
        self.iterator(TXN_TO_HEIGHT_SORT, anchor)
    }

    /////////////////////////
    // User command counts //
    /////////////////////////

    fn get_pk_num_user_commands_blocks(&self, pk: &PublicKey) -> anyhow::Result<Option<u32>> {
        trace!("Getting number of user commands for {pk}");
        Ok(self.get(USER_COMMANDS_PK_NUM, pk))
    }

    fn get_user_commands_epoch_count(&self, epoch: Option<u32>) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting user command epoch {epoch}");
        Ok(self.get(USER_COMMANDS_EPOCH, epoch))
    }

    fn increment_user_commands_epoch_count(&self, epoch: u32) -> anyhow::Result<()> {
        trace!("Incrementing user command epoch {epoch}");
        let old = self.get_user_commands_epoch_count(Some(epoch))?;
        Ok(self.put(USER_COMMANDS_EPOCH, epoch, old + 1))
    }

    fn get_user_commands_total_count(&self) -> anyhow::Result<u32> {
        trace!("Getting user command total");
        Ok(self
            .database
            .get(Self::TOTAL_NUM_USER_COMMANDS_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn increment_user_commands_total_count(&self) -> anyhow::Result<()> {
        trace!("Incrementing user command total");

        let old = self.get_user_commands_total_count()?;
        Ok(self
            .database
            .put(Self::TOTAL_NUM_USER_COMMANDS_KEY, to_be_bytes(old + 1))?)
    }

    fn get_user_commands_pk_epoch_count(
        &self,
        pk: &PublicKey,
        epoch: Option<u32>,
    ) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting user command epoch {epoch} num {pk}");
        Ok(self.get(USER_COMMANDS_PK_EPOCH, (epoch, &pk)))
    }

    fn increment_user_commands_pk_epoch_count(
        &self,
        pk: &PublicKey,
        epoch: u32,
    ) -> anyhow::Result<()> {
        trace!("Incrementing pk epoch {epoch} user commands count {pk}");

        let old = self.get_user_commands_pk_epoch_count(pk, Some(epoch))?;
        Ok(self.put(USER_COMMANDS_PK_EPOCH, (epoch, &pk), old + 1))
    }

    fn get_user_commands_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<u32> {
        trace!("Getting pk total user commands count {pk}");
        Ok(self.get(USER_COMMANDS_PK_TOTAL, pk))
    }

    fn increment_user_commands_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<()> {
        trace!("Incrementing user command pk total num {pk}");

        let old = self.get_user_commands_pk_total_count(pk)?;
        Ok(self.put(USER_COMMANDS_PK_TOTAL, pk, old + 1))
    }

    fn set_block_user_commands_count(
        &self,
        state_hash: &BlockHash,
        count: u32,
    ) -> anyhow::Result<()> {
        trace!("Setting block user command count {state_hash} -> {count}");
        Ok(self.put(BLOCK_USER_COMMAND_COUNTS, state_hash, count)?)
    }

    fn get_block_user_commands_count(&self, state_hash: &BlockHash) -> anyhow::Result<Option<u32>> {
        trace!("Getting block user command count {state_hash}");
        Ok(self.get(BLOCK_USER_COMMAND_COUNTS, state_hash))
    }

    fn increment_user_commands_counts(
        &self,
        command: &UserCommandWithStatus,
        epoch: u32,
    ) -> anyhow::Result<()> {
        trace!(
            "Incrementing user commands counts {:?}",
            command.to_command()
        );

        // sender epoch & total
        let sender = command.sender();
        self.increment_user_commands_pk_epoch_count(&sender, epoch)?;
        self.increment_user_commands_pk_total_count(&sender)?;

        // receiver epoch & total
        let receiver = command.receiver();
        if sender != receiver {
            self.increment_user_commands_pk_epoch_count(&receiver, epoch)?;
            self.increment_user_commands_pk_total_count(&receiver)?;
        }

        // epoch & total counts
        self.increment_user_commands_epoch_count(epoch)?;
        self.increment_user_commands_total_count()
    }
}
