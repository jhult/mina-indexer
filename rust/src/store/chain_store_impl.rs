use super::{database::CHAIN_ID_TO_NETWORK, fixed_keys::FixedKeys};
use crate::{
    chain::{store::ChainStore, ChainId, Network},
    store::{database::STRING_KEYS, IndexerStore},
};
use log::trace;

impl ChainStore for IndexerStore {
    fn set_chain_id_for_network(
        &self,
        chain_id: &ChainId,
        network: &Network,
    ) -> anyhow::Result<()> {
        trace!(
            "Setting chain id '{}' for network '{}'",
            chain_id.0,
            network
        );

        // add the new pair
        self.put(CHAIN_ID_TO_NETWORK, chain_id, network);

        // update current chain_id
        self.database
            .write(STRING_KEYS, Self::CHAIN_ID_KEY, &chain_id.0);
        Ok(())
    }

    fn get_network(&self, chain_id: &ChainId) -> anyhow::Result<Network> {
        trace!("Getting network for chain id: {}", chain_id.0);
        Ok(self.get(CHAIN_ID_TO_NETWORK, chain_id))
    }

    fn get_current_network(&self) -> anyhow::Result<Network> {
        trace!("Getting current network");
        self.get_network(&self.get_chain_id()?)
    }

    fn get_chain_id(&self) -> anyhow::Result<ChainId> {
        trace!("Getting chain id");
        Ok(ChainId(self.get(STRING_KEYS, Self::CHAIN_ID_KEY)))
    }
}
