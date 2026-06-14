use serde_json::{Map, Value};

#[path = "openapi_schemas_health.rs"]
mod openapi_schemas_health;
#[path = "openapi_schemas_inbox.rs"]
mod openapi_schemas_inbox;
#[path = "openapi_schemas_provider.rs"]
mod openapi_schemas_provider;
#[path = "openapi_schemas_scoring.rs"]
mod openapi_schemas_scoring;

pub(super) fn core_schemas() -> Value {
    let mut schemas = Map::new();
    append_schemas(&mut schemas, openapi_schemas_inbox::inbox_schemas());
    append_schemas(&mut schemas, openapi_schemas_scoring::scoring_schemas());
    append_schemas(&mut schemas, openapi_schemas_provider::provider_schemas());
    append_schemas(&mut schemas, openapi_schemas_health::health_schemas());
    Value::Object(schemas)
}

fn append_schemas(target: &mut Map<String, Value>, schemas: Value) {
    let Value::Object(schemas) = schemas else {
        unreachable!("OpenAPI core schema group must be a JSON object");
    };
    target.extend(schemas);
}
