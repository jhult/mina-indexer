pub mod blocks;
mod gen;
mod query_data;
mod transactions;

use self::gen::{schema_builder, Mutation, Query};
use crate::store::IndexerStore;
use actix_web::{http::header::ContentType, HttpResponse};
use async_graphql::{http::GraphiQLSource, Context, EmptySubscription, Schema};
use chrono::{DateTime, SecondsFormat};
use std::sync::Arc;

pub const ENDPOINT_GRAPHQL: &str = "/graphql";

pub(crate) async fn graphiql() -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(GraphiQLSource::build().endpoint(ENDPOINT_GRAPHQL).finish()))
}

pub(crate) fn build_schema(store: Arc<IndexerStore>) -> Schema<Query, Mutation, EmptySubscription> {
    schema_builder().data(store).finish()
}

pub(crate) fn db<'ctx>(ctx: &Context) -> &'ctx Arc<IndexerStore> {
    ctx.data::<Arc<IndexerStore>>()
        .expect("Database should be in the context")
}

/// convert epoch millis to an ISO 8601 formatted date
pub(crate) fn millis_to_date_string(millis: i64) -> String {
    let date_time = DateTime::from_timestamp_millis(millis).unwrap();
    date_time.to_rfc3339_opts(SecondsFormat::Millis, true)
}

// JSON utility
pub(crate) fn sanitize_json<T: serde::Serialize>(s: T) -> String {
    serde_json::to_string(&s).unwrap().replace('\"', "")
}

pub(crate) fn sanitize_json_option<T: serde::Serialize>(s: T) -> Option<String> {
    Option::from(sanitize_json(s))
}
