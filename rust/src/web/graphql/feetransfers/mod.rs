use super::{
    gen::{Feetransfer, FeetransferQueryInput, FeetransferSortByInput, Query},
    DataSource,
};
use async_graphql::{Context, Result};

impl DataSource {
    pub async fn query_feetransfer(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _input: FeetransferQueryInput,
    ) -> Result<Option<Feetransfer>> {
        todo!()
    }

    pub async fn query_feetransfers(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _input: FeetransferQueryInput,
        _limit: Option<i64>,
        _sort_by: FeetransferSortByInput,
    ) -> Result<Vec<Option<Feetransfer>>> {
        todo!()
    }
}
