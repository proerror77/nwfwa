use serde_json::{json, Map, Value};

#[path = "openapi_paths_core.rs"]
mod openapi_paths_core;
#[path = "openapi_paths_data_ops.rs"]
mod openapi_paths_data_ops;
#[path = "openapi_paths_governance.rs"]
mod openapi_paths_governance;
#[path = "openapi_paths_model_ops.rs"]
mod openapi_paths_model_ops;
#[path = "openapi_paths_pilot.rs"]
mod openapi_paths_pilot;
#[path = "openapi_paths_rules.rs"]
mod openapi_paths_rules;

pub(super) fn openapi_paths() -> Value {
    let mut paths = Map::new();
    append_paths(&mut paths, openapi_paths_core::core_paths());
    append_paths(&mut paths, openapi_paths_rules::rule_paths());
    append_paths(&mut paths, openapi_paths_data_ops::data_ops_paths());
    append_paths(&mut paths, openapi_paths_governance::governance_paths());
    append_paths(&mut paths, openapi_paths_model_ops::model_ops_paths());
    append_paths(&mut paths, openapi_paths_pilot::pilot_paths());
    Value::Object(paths)
}

fn append_paths(target: &mut Map<String, Value>, paths: Value) {
    let Value::Object(paths) = paths else {
        unreachable!("OpenAPI path group must be a JSON object");
    };
    target.extend(paths);
}

fn routing_policy_lifecycle_parameters() -> Value {
    json!([
        {
            "name": "policy_id",
            "in": "path",
            "required": true,
            "schema": { "type": "string" }
        },
        {
            "name": "review_mode",
            "in": "path",
            "required": true,
            "schema": { "type": "string", "enum": ["pre_payment", "post_payment", "both"] }
        },
        {
            "name": "version",
            "in": "path",
            "required": true,
            "schema": { "type": "integer", "minimum": 1 }
        }
    ])
}

fn rule_lifecycle_parameters() -> Value {
    json!([
        {
            "name": "rule_id",
            "in": "path",
            "required": true,
            "schema": { "type": "string" }
        }
    ])
}

fn rule_lifecycle_request_body() -> Value {
    json!({
        "required": true,
        "content": {
            "application/json": {
                "schema": { "$ref": "#/components/schemas/RuleLifecycleRequest" }
            }
        }
    })
}

fn routing_policy_lifecycle_request_body() -> Value {
    json!({
        "required": true,
        "content": {
            "application/json": {
                "schema": { "$ref": "#/components/schemas/RoutingPolicyLifecycleRequest" }
            }
        }
    })
}

fn model_lifecycle_request_body() -> Value {
    json!({
        "required": true,
        "content": {
            "application/json": {
                "schema": { "$ref": "#/components/schemas/ModelLifecycleRequest" }
            }
        }
    })
}
