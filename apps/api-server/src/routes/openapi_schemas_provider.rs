use serde_json::{Map, Value};

#[path = "openapi_schemas_provider_anomaly.rs"]
mod openapi_schemas_provider_anomaly;
#[path = "openapi_schemas_provider_medical.rs"]
mod openapi_schemas_provider_medical;
#[path = "openapi_schemas_provider_profile.rs"]
mod openapi_schemas_provider_profile;
#[path = "openapi_schemas_provider_scoring.rs"]
mod openapi_schemas_provider_scoring;

pub(super) fn provider_schemas() -> Value {
    let mut schemas = Map::new();
    append_schemas(
        &mut schemas,
        openapi_schemas_provider_profile::provider_profile_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_provider_anomaly::provider_anomaly_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_provider_medical::provider_medical_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_provider_scoring::provider_scoring_schemas(),
    );
    Value::Object(schemas)
}

fn append_schemas(target: &mut Map<String, Value>, schemas: Value) {
    let Value::Object(schemas) = schemas else {
        unreachable!("OpenAPI provider schema group must be a JSON object");
    };
    target.extend(schemas);
}
