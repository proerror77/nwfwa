use serde_json::{Map, Value};

#[path = "openapi_schemas_rules_candidate_review.rs"]
mod openapi_schemas_rules_candidate_review;
#[path = "openapi_schemas_rules_catalog.rs"]
mod openapi_schemas_rules_catalog;
#[path = "openapi_schemas_rules_definition.rs"]
mod openapi_schemas_rules_definition;
#[path = "openapi_schemas_rules_lifecycle.rs"]
mod openapi_schemas_rules_lifecycle;
#[path = "openapi_schemas_rules_observability.rs"]
mod openapi_schemas_rules_observability;
#[path = "openapi_schemas_rules_promotion.rs"]
mod openapi_schemas_rules_promotion;

pub(super) fn rule_schemas() -> Value {
    let mut schemas = Map::new();
    append_schemas(
        &mut schemas,
        openapi_schemas_rules_catalog::catalog_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_rules_lifecycle::lifecycle_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_rules_candidate_review::candidate_review_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_rules_definition::definition_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_rules_promotion::promotion_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_rules_observability::observability_schemas(),
    );
    Value::Object(schemas)
}

fn append_schemas(target: &mut Map<String, Value>, schemas: Value) {
    let Value::Object(schemas) = schemas else {
        unreachable!("OpenAPI rule schema group must be a JSON object");
    };
    target.extend(schemas);
}
