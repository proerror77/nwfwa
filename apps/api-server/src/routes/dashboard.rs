use crate::{
    app::AppState, auth::AuthenticatedActor, error::ApiError, repository::DashboardSummaryRecord,
};
use axum::{extract::State, Json};

pub async fn dashboard_summary(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
) -> Result<Json<DashboardSummaryRecord>, ApiError> {
    let summary = state
        .repository
        .dashboard_summary(Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("DASHBOARD_SUMMARY_FAILED"))?;
    Ok(Json(summary))
}

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::internal(code, error)
}
