use serde_json::{Map, Value};

#[path = "openapi_schemas_data_models_datasets.rs"]
mod openapi_schemas_data_models_datasets;
#[path = "openapi_schemas_data_models_evidence.rs"]
mod openapi_schemas_data_models_evidence;
#[path = "openapi_schemas_data_models_factors.rs"]
mod openapi_schemas_data_models_factors;
#[path = "openapi_schemas_data_models_model_governance.rs"]
mod openapi_schemas_data_models_model_governance;

pub(super) fn data_model_schemas() -> Value {
    let mut schemas = Map::new();
    append_schemas(
        &mut schemas,
        openapi_schemas_data_models_datasets::dataset_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_data_models_evidence::evidence_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_data_models_factors::factor_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_data_models_model_governance::model_governance_schemas(),
    );
    Value::Object(schemas)
}

fn append_schemas(target: &mut Map<String, Value>, schemas: Value) {
    let Value::Object(schemas) = schemas else {
        unreachable!("OpenAPI data model schema group must be a JSON object");
    };
    target.extend(schemas);
}
