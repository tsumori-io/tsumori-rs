use axum::{
    body::{Body, Bytes},
    extract::{Json, Path, Query, Request, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    convert::Infallible,
    future::ready,
    sync::OnceLock,
    time::{Duration, Instant},
};
use tower_http::{
    cors::{Any, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::Span;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn router() -> Router {
    Router::new()
        .route("/chains", get(get_chains))
        .route("/orders/:account", get(get_account_orders))
        .route("/quote", get(get_quote))
}

async fn get_chains() -> impl IntoResponse {
    let chain_data: Vec<_> = utils::get_supported_chains()
        .iter()
        .map(|(_, &chaindata)| json!(chaindata))
        .collect();

    let response = Json(json!({ "chains": chain_data }));
    (StatusCode::OK, response)
}

async fn get_account_orders(Path(account): Path<String>) -> impl IntoResponse {
    // TODO: get bridging tx's for account from db
}

#[derive(Debug, serde::Deserialize)]
struct QuoteParams {
    id: u64,
}

async fn get_quote(Query(params): Query<QuoteParams>) -> impl IntoResponse {
    let quote = Json(json!({
      "quote": "Do not go gentle into that good night.",
      "author": "Dylan Thomas",
    }));

    (StatusCode::OK, quote)
}
