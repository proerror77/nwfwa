use serde_json::{Map, Value};

#[path = "openapi_paths_data_ops_datasets.rs"]
mod openapi_paths_data_ops_datasets;
#[path = "openapi_paths_data_ops_evidence.rs"]
mod openapi_paths_data_ops_evidence;
#[path = "openapi_paths_data_ops_model_assets.rs"]
mod openapi_paths_data_ops_model_assets;
#[path = "openapi_paths_data_ops_operations.rs"]
mod openapi_paths_data_ops_operations;

pub(super) fn data_ops_paths() -> Value {
    let mut paths = Map::new();
    append_paths(&mut paths, openapi_paths_data_ops_datasets::dataset_paths());
    append_paths(
        &mut paths,
        openapi_paths_data_ops_model_assets::model_asset_paths(),
    );
    append_paths(
        &mut paths,
        openapi_paths_data_ops_evidence::evidence_paths(),
    );
    append_paths(
        &mut paths,
        openapi_paths_data_ops_operations::operational_paths(),
    );
    Value::Object(paths)
}

fn append_paths(target: &mut Map<String, Value>, paths: Value) {
    let Value::Object(paths) = paths else {
        unreachable!("OpenAPI data ops path group must be a JSON object");
    };
    target.extend(paths);
}
