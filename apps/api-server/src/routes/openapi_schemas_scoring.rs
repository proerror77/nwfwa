use serde_json::{Map, Value};

#[path = "openapi_schemas_scoring_requests.rs"]
mod openapi_schemas_scoring_requests;
#[path = "openapi_schemas_scoring_responses.rs"]
mod openapi_schemas_scoring_responses;

pub(super) fn scoring_schemas() -> Value {
    let mut schemas = Map::new();
    append_schemas(
        &mut schemas,
        openapi_schemas_scoring_requests::scoring_request_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_scoring_responses::scoring_response_schemas(),
    );
    Value::Object(schemas)
}

fn append_schemas(target: &mut Map<String, Value>, schemas: Value) {
    let Value::Object(schemas) = schemas else {
        unreachable!("OpenAPI scoring schema group must be a JSON object");
    };
    target.extend(schemas);
}
