use serde_json::{Map, Value};

#[path = "openapi_schemas_data_models_mlops_queues.rs"]
mod openapi_schemas_data_models_mlops_queues;
#[path = "openapi_schemas_data_models_model_catalog.rs"]
mod openapi_schemas_data_models_model_catalog;
#[path = "openapi_schemas_data_models_model_promotion.rs"]
mod openapi_schemas_data_models_model_promotion;
#[path = "openapi_schemas_data_models_retraining.rs"]
mod openapi_schemas_data_models_retraining;
#[path = "openapi_schemas_data_models_routing_policies.rs"]
mod openapi_schemas_data_models_routing_policies;

pub(super) fn model_governance_schemas() -> Value {
    let mut schemas = Map::new();
    append_schemas(
        &mut schemas,
        openapi_schemas_data_models_model_catalog::model_catalog_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_data_models_routing_policies::routing_policy_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_data_models_model_promotion::model_promotion_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_data_models_mlops_queues::mlops_queue_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_data_models_retraining::retraining_schemas(),
    );
    Value::Object(schemas)
}

fn append_schemas(target: &mut Map<String, Value>, schemas: Value) {
    let Value::Object(schemas) = schemas else {
        unreachable!("OpenAPI model governance schema group must be a JSON object");
    };
    target.extend(schemas);
}
