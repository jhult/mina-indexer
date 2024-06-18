use crate::{
    block::BlockHash,
    ledger::{public_key::PublicKey, username::Username},
    store::{
        database::{USERNAMES_PER_BLOCK, USERNAME_PK_INDEX, USERNAME_PK_NUM},
        to_be_bytes,
        username::UsernameUpdate,
        DBUpdate,
    },
};

use super::{
    username::{UsernameAccountUpdate, UsernameStore},
    IndexerStore,
};
use log::{error, trace};
use std::collections::HashMap;

impl UsernameStore for IndexerStore {
    fn get_username(&self, pk: &PublicKey) -> anyhow::Result<Option<Username>> {
        trace!("Getting {pk} username");
        if let Ok(Some(index)) = self.get_pk_num_username_updates(pk) {
            return self.get_pk_username(pk, index);
        }
        Ok(None)
    }

    fn set_block_username_updates(
        &self,
        state_hash: &BlockHash,
        username_updates: &UsernameUpdate,
    ) -> anyhow::Result<()> {
        trace!("Setting block username updates {state_hash}");
        Ok(self.put(USERNAMES_PER_BLOCK, state_hash, username_updates)?)
    }

    fn get_block_username_updates(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<HashMap<PublicKey, Username>>> {
        trace!("Getting block username updates {state_hash}");
        Ok(self.get(USERNAMES_PER_BLOCK, state_hash))
    }

    fn reorg_username_updates(
        &self,
        old_best_tip: &BlockHash,
        new_best_tip: &BlockHash,
    ) -> anyhow::Result<UsernameAccountUpdate> {
        trace!("Getting common username updates:\n  old: {old_best_tip}\n  new: {new_best_tip}");

        // follows the old best tip back to the common ancestor
        let mut a = old_best_tip.clone();
        let mut unapply = vec![];

        // follows the new best tip back to the common ancestor
        let mut b = new_best_tip.clone();
        let mut apply = vec![];

        let a_length = self.get_block_height(&a)?.expect("a has a length");
        let b_length = self.get_block_height(&b)?.expect("b has a length");

        // bring b back to the same height as a
        let genesis_state_hashes: Vec<BlockHash> = self.get_known_genesis_state_hashes()?;
        for _ in 0..(b_length - a_length) {
            // check if there's a previous block
            if genesis_state_hashes.contains(&b) {
                break;
            }

            apply.push(UsernameUpdate(
                self.get_block_username_updates(&b)?.unwrap(),
            ));
            b = self.get_block_parent_hash(&b)?.expect("b has a parent");
        }

        // find the common ancestor
        let mut a_prev = self.get_block_parent_hash(&a)?.expect("a has a parent");
        let mut b_prev = self.get_block_parent_hash(&b)?.expect("b has a parent");

        while a != b && !genesis_state_hashes.contains(&a) {
            // add blocks to appropriate collection
            unapply.push(UsernameUpdate(
                self.get_block_username_updates(&a)?.unwrap(),
            ));
            apply.push(UsernameUpdate(
                self.get_block_username_updates(&b)?.unwrap(),
            ));

            // descend
            a = a_prev;
            b = b_prev;

            a_prev = self.get_block_parent_hash(&a)?.expect("a has a parent");
            b_prev = self.get_block_parent_hash(&b)?.expect("b has a parent");
        }

        apply.reverse();
        Ok(DBUpdate { apply, unapply })
    }

    fn update_usernames(&self, update: UsernameAccountUpdate) -> anyhow::Result<()> {
        trace!("Updating usernames");

        // unapply
        for updates in update.unapply {
            for pk in updates.0.keys() {
                if let Some(num) = self.get_pk_num_username_updates(pk)? {
                    // decr pk num username updates
                    if num == 0 {
                        // remove pk number
                        self.dete(USERNAME_PK_NUM, pk)?;

                        // remove pk index
                        let mut key = pk.clone().to_bytes();
                        key.append(&mut to_be_bytes(0));
                        self.delete(USERNAME_PK_INDEX, key)?;
                    }
                    self.put(USERNAME_PK_NUM, pk, num - 1)?;

                    // drop last update
                    let mut key = pk.clone().to_bytes();
                    key.append(&mut to_be_bytes(num));
                    self.delete(USERNAME_PK_INDEX, key)?;
                } else {
                    error!("Invalid username pk num {pk}");
                }
            }
        }

        // apply
        for updates in update.apply {
            for (pk, username) in updates.0 {
                if let Some(mut num) = self.get_pk_num_username_updates(&pk)? {
                    // incr pk num username updates
                    num += 1;
                    self.put(USERNAME_PK_NUM, pk, num)?;

                    // add update
                    let mut key = pk.to_bytes();
                    key.append(&mut to_be_bytes(num));
                    self.put(USERNAME_PK_INDEX, key, username.0.as_bytes())?;
                } else {
                    self.put(USERNAME_PK_NUM, pk, 0)?;

                    // add update
                    let mut key = pk.to_bytes();
                    key.append(&mut to_be_bytes(0));
                    self.put(USERNAME_PK_INDEX, key, username.0.as_bytes())?;
                }
            }
        }
        Ok(())
    }

    fn get_pk_username(&self, pk: &PublicKey, index: u32) -> anyhow::Result<Option<Username>> {
        trace!("Getting pk's {index}th username {pk}");
        let mut key = pk.clone().to_bytes();
        key.append(&mut to_be_bytes(index));
        Ok(self.get(USERNAME_PK_INDEX, key))
    }

    fn get_pk_num_username_updates(&self, pk: &PublicKey) -> anyhow::Result<Option<u32>> {
        trace!("Getting pk's number of username updates {pk}");
        Ok(self.get(USERNAME_PK_NUM, pk))
    }
}
