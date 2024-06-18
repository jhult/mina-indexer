use crate::{
    block::{store::BlockStore, BlockHash},
    canonicity::store::CanonicityStore,
    constants::*,
    ledger::{diff::account::PaymentDiff, public_key::PublicKey},
    store::{
        account::AccountStore,
        database::{ACCOUNT_BALANCE, ACCOUNT_BALANCE_SORT, ACCOUNT_BALANCE_UPDATES},
        u64_prefix_key, DBUpdate, IndexerStore,
    },
};
use log::trace;
use std::collections::HashSet;

use super::{DBIterator, IteratorAnchor};

impl AccountStore for IndexerStore {
    fn reorg_account_balance_updates(
        &self,
        old_best_tip: &BlockHash,
        new_best_tip: &BlockHash,
    ) -> anyhow::Result<(Vec<PaymentDiff>, HashSet<PublicKey>)> {
        trace!(
            "Getting common ancestor account balance updates:\n  old: {}\n  new: {}",
            old_best_tip,
            new_best_tip
        );
        let mut coinbase_receivers = HashSet::new();

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

            coinbase_receivers.insert(self.get_coinbase_receiver(&b)?.expect("b has a coinbase"));
            apply.append(&mut self.get_block_balance_updates(&b)?.unwrap().1);
            b = self.get_block_parent_hash(&b)?.expect("b has a parent");
        }

        // find the common ancestor
        let mut a_prev = self.get_block_parent_hash(&a)?.expect("a has a parent");
        let mut b_prev = self.get_block_parent_hash(&b)?.expect("b has a parent");

        while a != b && !genesis_state_hashes.contains(&a) {
            // retain coinbase receivers
            coinbase_receivers.insert(self.get_coinbase_receiver(&a)?.expect("a has a coinbase"));
            coinbase_receivers.insert(self.get_coinbase_receiver(&b)?.expect("b has a coinbase"));

            // add blocks to appropriate collection
            unapply.append(&mut self.get_block_balance_updates(&a)?.unwrap().1);
            apply.append(&mut self.get_block_balance_updates(&b)?.unwrap().1);

            // descend
            a = a_prev;
            b = b_prev;

            a_prev = self.get_block_parent_hash(&a)?.expect("a has a parent");
            b_prev = self.get_block_parent_hash(&b)?.expect("b has a parent");
        }

        // balance updates don't require this reverse, but other updates may
        apply.reverse();
        Ok((
            <DBUpdate<PaymentDiff>>::new(apply, unapply).to_diff_vec(),
            coinbase_receivers,
        ))
    }

    fn get_block_balance_updates(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<(PublicKey, Vec<PaymentDiff>)>> {
        trace!("Getting block balance updates for {state_hash}");
        Ok(self.get(ACCOUNT_BALANCE_UPDATES, state_hash))
    }

    fn update_account_balances(
        &self,
        state_hash: &BlockHash,
        updates: Vec<PaymentDiff>,
        coinbase_receivers: HashSet<PublicKey>,
    ) -> anyhow::Result<()> {
        trace!("Updating account balances {state_hash}");

        // update balances
        for (pk, amount) in <DBUpdate<PaymentDiff>>::balance_updates(updates) {
            if amount != 0 {
                let pk = pk.into();
                let balance = self.get_account_balance(&pk)?.unwrap_or(0);
                let balance = if coinbase_receivers.contains(&pk) && balance == 0 && amount > 0 {
                    (balance + amount.unsigned_abs()).saturating_sub(MAINNET_ACCOUNT_CREATION_FEE.0)
                } else if amount > 0 {
                    balance + amount.unsigned_abs()
                } else {
                    balance.saturating_sub(amount.unsigned_abs())
                };

                // coinbase receivers may need to be removed
                self.update_account_balance(
                    &pk,
                    if coinbase_receivers.contains(&pk) && balance == 0 {
                        None
                    } else {
                        Some(balance)
                    },
                )?;
            }
        }
        Ok(())
    }

    fn update_account_balance(&self, pk: &PublicKey, balance: Option<u64>) -> anyhow::Result<()> {
        trace!("Updating account balance {pk} -> {balance:?}");

        // update balance info
        if balance.is_none() {
            // delete stale data
            let b = self.get_account_balance(pk)?.unwrap_or(0);
            self.delete(ACCOUNT_BALANCE, pk)?;
            self.delete(ACCOUNT_BALANCE_SORT, u64_prefix_key(b, &pk.0))?;
            return Ok(());
        }

        let balance = balance.unwrap();
        if let Some(old) = self.get_account_balance(pk)? {
            // delete stale balance sorting data
            self.database
                .delete_cf(self.account_balance_sort_cf(), u64_prefix_key(old, &pk.0))?;
        }
        self.put(ACCOUNT_BALANCE, pk, balance);

        // add: {balance}{pk} -> _
        self.database.put_cf(
            self.account_balance_sort_cf(),
            u64_prefix_key(balance, &pk.0),
            b"",
        )?;
        Ok(())
    }

    fn set_block_balance_updates(
        &self,
        state_hash: &BlockHash,
        coinbase_receiver: PublicKey,
        balance_updates: Vec<PaymentDiff>,
    ) -> anyhow::Result<()> {
        trace!("Setting block balance updates for {state_hash}");
        self.put(
            ACCOUNT_BALANCE_UPDATES,
            state_hash,
            &(coinbase_receiver, balance_updates),
        );
        Ok(())
    }

    fn get_account_balance(&self, pk: &PublicKey) -> anyhow::Result<Option<u64>> {
        trace!("Getting account balance {pk}");

        Ok(self
            .database
            .get_pinned_cf(self.account_balance_cf(), pk.0.as_bytes())?
            .map(|bytes| {
                let mut be_bytes = [0; 8];
                be_bytes.copy_from_slice(&bytes[..8]);
                u64::from_be_bytes(be_bytes)
            }))
    }

    ///////////////
    // Iterators //
    ///////////////

    fn account_balance_iterator<'a>(&'a self, mode: IteratorAnchor) -> DBIterator<K, V> {
        self.database
            .iterator_cf(self.account_balance_sort_cf(), mode)
    }
}
