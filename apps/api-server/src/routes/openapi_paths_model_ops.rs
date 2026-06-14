use serde_json::{Map, Value};

#[path = "openapi_paths_model_ops_core.rs"]
mod openapi_paths_model_ops_core;
#[path = "openapi_paths_model_ops_lifecycle.rs"]
mod openapi_paths_model_ops_lifecycle;
#[path = "openapi_paths_model_ops_mlops.rs"]
mod openapi_paths_model_ops_mlops;
#[path = "openapi_paths_model_ops_providers.rs"]
mod openapi_paths_model_ops_providers;
#[path = "openapi_paths_model_ops_retraining.rs"]
mod openapi_paths_model_ops_retraining;
#[path = "openapi_paths_model_ops_review.rs"]
mod openapi_paths_model_ops_review;

pub(super) fn model_ops_paths() -> Value {
    let mut paths = Map::new();
    append_paths(
        &mut paths,
        openapi_paths_model_ops_providers::provider_paths(),
    );
    append_paths(&mut paths, openapi_paths_model_ops_review::review_paths());
    append_paths(&mut paths, openapi_paths_model_ops_core::model_core_paths());
    append_paths(&mut paths, openapi_paths_model_ops_mlops::mlops_paths());
    append_paths(
        &mut paths,
        openapi_paths_model_ops_retraining::retraining_paths(),
    );
    append_paths(
        &mut paths,
        openapi_paths_model_ops_lifecycle::lifecycle_paths(),
    );
    Value::Object(paths)
}

fn append_paths(target: &mut Map<String, Value>, paths: Value) {
    let Value::Object(paths) = paths else {
        unreachable!("OpenAPI model ops path group must be a JSON object");
    };
    target.extend(paths);
}
