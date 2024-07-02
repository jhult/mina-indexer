use super::{
    database::SNARK_TOP_PRODUCERS_SORT, fixed_keys::FixedKeys, DBIterator, IteratorAnchor,
};
use crate::store::u64_prefix_key;
use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore, BlockHash},
    ledger::public_key::PublicKey,
    snark_work::{
        store::SnarkStore, SnarkWorkSummary, SnarkWorkSummaryWithStateHash, SnarkWorkTotal,
    },
    store::{
        database::{
            BLOCK_SNARK_COUNTS, SNARKS, SNARKS_EPOCH, SNARKS_PK_EPOCH, SNARKS_PK_TOTAL,
            SNARK_TOP_PRODUCERS, SNARK_WORK_FEES, SNARK_WORK_PROVER,
        },
        from_be_bytes, to_be_bytes, IndexerStore,
    },
    web::graphql::snarks::SnarkWithCanonicity,
};
use log::trace;
use std::collections::HashMap;

/// **Key format:** `{fee}{slot}{pk}{hash}{num}`
/// ```
/// fee:  8 BE bytes
/// slot: 4 BE bytes
/// pk:   [PublicKey::LEN] bytes
/// hash: [BlockHash::LEN] bytes
/// num:  4 BE bytes
fn snark_fee_prefix_key(
    fee: u64,
    global_slot: u32,
    pk: PublicKey,
    state_hash: BlockHash,
    num: u32,
) -> Vec<u8> {
    let mut bytes = fee.to_be_bytes().to_vec();
    bytes.append(&mut global_slot.to_be_bytes().to_vec());
    bytes.append(&mut pk.0.as_bytes().to_vec());
    bytes.append(&mut state_hash.0.as_bytes().to_vec());
    bytes.append(&mut num.to_be_bytes().to_vec());
    bytes
}

impl SnarkStore for IndexerStore {
    fn add_snark_work(&self, block: &PrecomputedBlock) -> anyhow::Result<()> {
        trace!("Adding SNARK work from block {}", block.summary());

        let epoch = block.epoch_count();
        let global_slot = block.global_slot_since_genesis();
        let block_height = block.blockchain_length();
        let completed_works = SnarkWorkSummary::from_precomputed(block);
        let completed_works_state_hash = SnarkWorkSummaryWithStateHash::from_precomputed(block);

        // add: state hash -> snark works
        let state_hash = block.state_hash();
        self.put(SNARKS, state_hash, completed_works);

        // per block SNARK count
        self.set_block_snarks_count(&block.state_hash(), completed_works.len() as u32)?;

        // store fee info
        let mut num_prover_works: HashMap<PublicKey, u32> = HashMap::new();
        for snark in completed_works {
            let num = num_prover_works.get(&snark.prover).copied().unwrap_or(0);
            self.put(
                SNARK_WORK_FEES,
                (
                    snark.fee,
                    block.global_slot_since_genesis(),
                    snark.prover.clone(),
                    block.state_hash(),
                ),
                num,
            );

            // build the block's fee table
            if num_prover_works.get(&snark.prover).is_some() {
                *num_prover_works.get_mut(&snark.prover).unwrap() += 1;
            } else {
                num_prover_works.insert(snark.prover.clone(), 1);
            }
        }

        // add: "pk -> linked list of SNARK work summaries with state hash"
        for pk in block.prover_keys() {
            let pk_str = pk.to_address();
            trace!("Adding SNARK work for pk {pk}");

            // get pk's next index
            let n = self.get_pk_num_prover_blocks(&pk_str)?.unwrap_or(0);

            let block_pk_snarks: Vec<SnarkWorkSummaryWithStateHash> = completed_works_state_hash
                .clone()
                .into_iter()
                .filter(|snark| snark.contains_pk(&pk))
                .collect();

            if !block_pk_snarks.is_empty() {
                // write these SNARKs to the next key for pk
                let key = format!("{pk_str}{n}").as_bytes().to_vec();
                let value = serde_json::to_vec(&block_pk_snarks)?;
                self.put(self.snarks_cf(), key, value)?;

                // update pk's next index
                let key = pk_str.as_bytes();
                let next_n = (n + 1).to_string();
                let value = next_n.as_bytes();
                self.put(self.snarks_cf(), key, value)?;

                // increment SNARK counts
                for (index, snark) in block_pk_snarks.iter().enumerate() {
                    if self
                        .get(SNARK_WORK_PROVER, (&pk, global_slot, index as u32))?
                        .is_none()
                    {
                        let snark: SnarkWorkSummary = snark.clone().into();
                        self.set_snark_by_prover(&snark, global_slot, index as u32)?;
                        self.set_snark_by_prover_height(&snark, block_height, index as u32)?;
                        self.increment_snarks_counts(&snark, epoch)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn get_pk_num_prover_blocks(&self, pk: &str) -> anyhow::Result<Option<u32>> {
        let key = pk.as_bytes();
        self.get(SNARKS, pk);
        Ok(self.get(self.snarks_cf(), key)?.and_then(|bytes| {
            String::from_utf8(bytes.to_vec())
                .ok()
                .and_then(|s| s.parse().ok())
        }))
    }

    fn get_snark_work_by_public_key(
        &self,
        pk: &PublicKey,
    ) -> anyhow::Result<Option<Vec<SnarkWorkSummaryWithStateHash>>> {
        let pk = pk.to_address();
        trace!("Getting SNARK work for public key {pk}");

        let mut all_snarks = None;
        fn key_n(pk: String, n: u32) -> Vec<u8> {
            format!("{pk}{n}").as_bytes().to_vec()
        }

        if let Some(n) = self.get_pk_num_prover_blocks(&pk)? {
            for m in 0..n {
                if let Some(mut block_m_snarks) = self
                    .database
                    // TODO: SNARKS TD
                    .get_pinned_cf(self.snarks_cf(), key_n(pk.clone(), m))?
                    .map(|bytes| {
                        serde_json::from_slice::<Vec<SnarkWorkSummaryWithStateHash>>(&bytes)
                            .expect("snark work with state hash")
                    })
                {
                    let mut snarks = all_snarks.unwrap_or(vec![]);
                    snarks.append(&mut block_m_snarks);
                    all_snarks = Some(snarks);
                } else {
                    all_snarks = None;
                    break;
                }
            }
        }
        Ok(all_snarks)
    }

    fn get_snark_work_in_block(
        &self,
        state_hash: &BlockHash,
    ) -> anyhow::Result<Option<Vec<SnarkWorkSummary>>> {
        trace!("Getting SNARK work in block {}", state_hash.0);

        Ok(self.get(SNARKS, state_hash))
    }

    fn update_top_snarkers(&self, snarks: Vec<SnarkWorkSummary>) -> anyhow::Result<()> {
        trace!("Updating top SNARK workers");

        let mut prover_fees: HashMap<PublicKey, (u64, u64)> = HashMap::new();
        for snark in snarks {
            let key = snark.prover;
            if prover_fees.get(&snark.prover).is_some() {
                prover_fees.get_mut(&snark.prover).unwrap().1 += snark.fee;
            } else {
                // TODO: where is this set??
                let old_total = self.get(SNARK_TOP_PRODUCERS, key).unwrap_or(0);
                prover_fees.insert(snark.prover.clone(), (old_total, snark.fee));

                // delete the stale data
                self.delete(
                    SNARK_TOP_PRODUCERS_SORT,
                    u64_prefix_key(old_total, &snark.prover.0),
                );
            }
        }

        // replace stale data with updated
        for (prover, (old_total, new_fees)) in prover_fees.iter() {
            let total_fees = old_total + new_fees;
            let key = u64_prefix_key(total_fees, &prover.0);
            self.put_sort(SNARK_TOP_PRODUCERS_SORT, key)?
        }

        Ok(())
    }

    fn get_top_snark_workers_by_fees(&self, n: usize) -> anyhow::Result<Vec<SnarkWorkTotal>> {
        trace!("Getting top {n} SNARK workers by fees");
        Ok(self
            .top_snark_workers_iterator(IteratorAnchor::End)
            .take(n)
            .map(|res| {
                res.map(|(bytes, _)| {
                    let mut total_fees_bytes = [0; 8];
                    total_fees_bytes.copy_from_slice(&bytes[..8]);
                    SnarkWorkTotal {
                        prover: PublicKey::from_bytes(&bytes[8..]).expect("public key"),
                        total_fees: u64::from_be_bytes(total_fees_bytes),
                    }
                })
                .expect("snark work iterator")
            })
            .collect())
    }

    fn top_snark_workers_iterator<'a>(
        &'a self,
        anchor: IteratorAnchor,
    ) -> DBIterator<u64, PublicKey> {
        self.iterator(SNARK_TOP_PRODUCERS_SORT, anchor)
    }

    fn snark_fees_iterator<'a>(
        &'a self,
        anchor: IteratorAnchor,
    ) -> DBIterator<(u64, u32, PublicKey, BlockHash), u32> {
        self.iterator(SNARK_WORK_FEES, anchor)
    }

    fn set_snark_by_prover(
        &self,
        snark: &SnarkWorkSummary,
        global_slot: u32,
        index: u32,
    ) -> anyhow::Result<()> {
        trace!(
            "Setting snark slot {global_slot} at index {index} for prover {}",
            snark.prover
        );
        Ok(self.put(
            SNARK_WORK_PROVER,
            (&snark.prover, global_slot, index),
            snark,
        ))
    }

    /// `{prover}{slot}{index} -> snark`
    /// - prover: 55 pk bytes
    /// - slot:   4 BE bytes
    /// - index:  4 BE bytes
    /// - snark:  serde_json encoded
    fn snark_prover_iterator<'a>(
        &'a self,
        anchor: IteratorAnchor,
    ) -> DBIterator<(PublicKey, u32, u32), SnarkWithCanonicity> {
        self.iterator(SNARK_WORK_PROVER, anchor)
    }

    fn set_snark_by_prover_height(
        &self,
        snark: &SnarkWorkSummary,
        block_height: u32,
        index: u32,
    ) -> anyhow::Result<()> {
        trace!(
            "Setting snark slot {block_height} at index {index} for prover {}",
            snark.prover
        );
        Ok(self.database.put_cf(
            self.snark_work_prover_height_cf(),
            snark_prover_prefix_key(&snark.prover, block_height, index),
            serde_json::to_vec(snark)?,
        )?)
    }

    /// `{prover}{height}{index} -> snark`
    /// - prover:         55 pk bytes
    /// - block height:   4 BE bytes
    /// - index:          4 BE bytes
    /// - snark:          serde_json encoded
    fn snark_prover_height_iterator<'a>(&'a self, mode: IteratorMode) -> DBIterator<'a> {
        self.database
            .iterator_cf(self.snark_work_prover_height_cf(), mode)
    }

    fn get_snarks_epoch_count(&self, epoch: Option<u32>) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting epoch {epoch} SNARKs count");
        Ok(self.get(SNARKS_EPOCH, epoch))
    }

    fn increment_snarks_epoch_count(&self, epoch: u32) -> anyhow::Result<()> {
        trace!("Incrementing epoch {epoch} SNARKs count");
        let old = self.get_snarks_epoch_count(Some(epoch))?;
        Ok(self.put(SNARKS_EPOCH, epoch, old + 1))
    }

    fn get_snarks_total_count(&self) -> anyhow::Result<u32> {
        trace!("Getting total SNARKs count");
        Ok(self
            .database
            .get(Self::TOTAL_NUM_SNARKS_KEY)?
            .map_or(0, from_be_bytes))
    }

    fn increment_snarks_total_count(&self) -> anyhow::Result<()> {
        trace!("Incrementing total SNARKs count");

        let old = self.get_snarks_total_count()?;
        Ok(self
            .database
            .put(Self::TOTAL_NUM_SNARKS_KEY, to_be_bytes(old + 1))?)
    }

    fn get_snarks_pk_epoch_count(&self, pk: &PublicKey, epoch: Option<u32>) -> anyhow::Result<u32> {
        let epoch = epoch.unwrap_or(self.get_current_epoch()?);
        trace!("Getting pk epoch {epoch} SNARKs count {pk}");
        Ok(self.get(SNARKS_PK_EPOCH, (epoch, pk)))
    }

    fn increment_snarks_pk_epoch_count(&self, pk: &PublicKey, epoch: u32) -> anyhow::Result<()> {
        trace!("Incrementing pk epoch {epoch} SNARKs count {pk}");

        let old = self.get_snarks_pk_epoch_count(pk, Some(epoch))?;
        Ok(self.put(SNARKS_PK_EPOCH, (epoch, pk), old + 1))
    }

    fn get_snarks_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<u32> {
        trace!("Getting pk total SNARKs count {pk}");
        Ok(self.get(SNARKS_PK_TOTAL, pk))
    }

    fn increment_snarks_pk_total_count(&self, pk: &PublicKey) -> anyhow::Result<()> {
        trace!("Incrementing pk total SNARKs count {pk}");

        let old = self.get_snarks_pk_total_count(pk)?;
        Ok(self.put(SNARKS_PK_TOTAL, pk, old + 1))
    }

    fn get_block_snarks_count(&self, state_hash: &BlockHash) -> anyhow::Result<Option<u32>> {
        trace!("Getting block SNARKs count {state_hash}");
        Ok(self.get(BLOCK_SNARK_COUNTS, state_hash))
    }

    fn set_block_snarks_count(&self, state_hash: &BlockHash, count: u32) -> anyhow::Result<()> {
        trace!("Setting block SNARKs count {state_hash} -> {count}");
        Ok(self.put(BLOCK_SNARK_COUNTS, state_hash, count)?)
    }

    fn increment_snarks_counts(&self, snark: &SnarkWorkSummary, epoch: u32) -> anyhow::Result<()> {
        trace!("Incrementing SNARKs count {snark:?}");

        // prover epoch & total
        let prover = snark.prover.clone();
        self.increment_snarks_pk_epoch_count(&prover, epoch)?;
        self.increment_snarks_pk_total_count(&prover)?;

        // epoch & total counts
        self.increment_snarks_epoch_count(epoch)?;
        self.increment_snarks_total_count()
    }
}
