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

pub fn router(state: crate::AppState) -> Router {
    Router::new()
        .route("/chains", get(get_chains))
        .route("/orders/:account", get(get_account_orders))
        .route("/tx", get(get_bridge_tx))
        .with_state(state)
}

async fn get_chains(
    State(crate::AppState { bridge_service }): State<crate::AppState>,
) -> impl IntoResponse {
    let chain_data = bridge_service.get_supported_chains();

    let response = Json(json!({ "chains": chain_data }));
    (StatusCode::OK, response)
}

async fn get_account_orders(Path(account): Path<String>) -> impl IntoResponse {
    // TODO: get bridging tx's for account from db
}

async fn get_bridge_tx(
    State(crate::AppState { bridge_service }): State<crate::AppState>,
    Query(params): Query<bridge::BridgeRequest>,
) -> impl IntoResponse {
    let response = bridge_service
        .get_tx(&params)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })
        .map(|res| (StatusCode::OK, Json(json!({ "response": res }))));
    response
}
