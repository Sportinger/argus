use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use tracing::{error, info, instrument};

use argus_core::api_types::{ReasoningApiResponse, ReasoningRequest};
use argus_core::reasoning::{ReasoningEngine, ReasoningQuery};

use crate::state::AppState;

#[instrument(skip(state), fields(question = %req.question))]
pub async fn query_reasoning(
    State(state): State<AppState>,
    Json(req): Json<ReasoningRequest>,
) -> impl IntoResponse {
    info!(
        context = req.context.as_deref().unwrap_or("none"),
        max_hops = req.max_hops,
        "Received reasoning query"
    );

    let query = ReasoningQuery {
        question: req.question,
        context: req.context,
        max_hops: req.max_hops,
    };

    match state.reasoning.query(&query).await {
        Ok(response) => {
            let api_response: ReasoningApiResponse = response.into();
            info!(
                confidence = api_response.confidence,
                steps = api_response.steps.len(),
                entities = api_response.entities_referenced.len(),
                "Reasoning query completed successfully"
            );
            Ok(Json(api_response))
        }
        Err(e) => {
            error!(error = %e, "Reasoning query failed");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Reasoning query failed: {e}")
                })),
            ))
        }
    }
}
