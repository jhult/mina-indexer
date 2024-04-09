use super::{
    gen::{Query, Snark, SnarkQueryInput, SnarkSortByInput},
    DataSource,
};
use async_graphql::{Context, Result};

impl DataSource {
    pub async fn query_snark(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _input: SnarkQueryInput,
    ) -> Result<Option<Snark>> {
        todo!()
    }

    pub async fn query_snarks(
        &self,
        _ctx: &Context<'_>,
        _p1: &Query,
        _input: SnarkQueryInput,
        _limit: Option<i64>,
        _sort_by: SnarkSortByInput,
    ) -> Result<Vec<Option<Snark>>> {
        todo!()
    }
}
