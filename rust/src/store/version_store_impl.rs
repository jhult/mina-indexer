use crate::store::database::INDEXER_STORE_VERSION;

use super::{
    fixed_keys::FixedKeys,
    version::{IndexerStoreVersion, VersionStore},
    IndexerStore,
};
use log::trace;

impl VersionStore for IndexerStore {
    /// Set db version with env var `GIT_COMMIT_HASH`
    fn set_db_version_with_git_commit(
        &self,
        major: u32,
        minor: u32,
        patch: u32,
    ) -> anyhow::Result<()> {
        let version = IndexerStoreVersion {
            major,
            minor,
            patch,
            ..Default::default()
        };
        trace!("Setting db version");
        if self
            .get(INDEXER_STORE_VERSION, Self::INDEXER_STORE_VERSION_KEY)
            .is_none()
        {
            self.put(
                INDEXER_STORE_VERSION,
                Self::INDEXER_STORE_VERSION_KEY,
                version,
            )
        }
        Ok(())
    }

    /// Get db version
    fn get_db_version(&self) -> anyhow::Result<IndexerStoreVersion> {
        trace!("Getting db version");
        Ok(self.get(INDEXER_STORE_VERSION, Self::INDEXER_STORE_VERSION_KEY))
    }
}
