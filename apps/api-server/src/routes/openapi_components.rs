use serde_json::{json, Map, Value};

#[path = "openapi_schemas_core.rs"]
mod openapi_schemas_core;
#[path = "openapi_schemas_data_models.rs"]
mod openapi_schemas_data_models;
#[path = "openapi_schemas_ops.rs"]
mod openapi_schemas_ops;
#[path = "openapi_schemas_rules.rs"]
mod openapi_schemas_rules;

pub(super) fn openapi_components() -> Value {
    json!({
        "securitySchemes": {
            "ApiKeyAuth": {
                "type": "apiKey",
                "in": "header",
                "name": "x-api-key"
            }
        },
        "schemas": merged_schemas()
    })
}

fn merged_schemas() -> Value {
    let mut schemas = Map::new();
    append_schemas(&mut schemas, openapi_schemas_core::core_schemas());
    append_schemas(&mut schemas, openapi_schemas_rules::rule_schemas());
    append_schemas(
        &mut schemas,
        openapi_schemas_data_models::data_model_schemas(),
    );
    append_schemas(&mut schemas, openapi_schemas_ops::ops_schemas());
    Value::Object(schemas)
}

fn append_schemas(target: &mut Map<String, Value>, schemas: Value) {
    let Value::Object(schemas) = schemas else {
        unreachable!("OpenAPI schema group must be a JSON object");
    };
    target.extend(schemas);
}
