mod accounts;
mod blocks;
mod feetransfers;
mod gen;
mod snarks;
mod stakes;
mod transactions;

use self::gen::{schema_builder, Query};
use super::{millis_to_iso_date_string, millis_to_rfc_date_string};
use crate::store::IndexerStore;
use actix_web::{http::header::ContentType, HttpResponse};
use async_graphql::{http::GraphiQLSource, Context, EmptyMutation, EmptySubscription, Schema};
use std::sync::Arc;

pub struct DataSource;

pub const ENDPOINT_GRAPHQL: &str = "/graphql";

pub(crate) fn build_schema(
    store: Arc<IndexerStore>,
) -> Schema<Query, EmptyMutation, EmptySubscription> {
    schema_builder().data(store).data(DataSource).finish()
}

pub(crate) async fn graphiql() -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(GraphiQLSource::build().endpoint(ENDPOINT_GRAPHQL).finish()))
}

pub(crate) fn db<'a>(ctx: &'a Context) -> &'a Arc<IndexerStore> {
    ctx.data::<Arc<IndexerStore>>()
        .expect("Database should be in the context")
}

// convert epoch milliseconds to an ISO 8601 formatted gen::DateTime scalar
pub(crate) fn date_time_to_scalar(millis: i64) -> gen::DateTime {
    gen::DateTime(millis_to_iso_date_string(millis))
}
