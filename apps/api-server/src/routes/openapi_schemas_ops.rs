use serde_json::{Map, Value};

#[path = "openapi_schemas_ops_agent_investigation.rs"]
mod openapi_schemas_ops_agent_investigation;
#[path = "openapi_schemas_ops_agent_runs.rs"]
mod openapi_schemas_ops_agent_runs;
#[path = "openapi_schemas_ops_cases.rs"]
mod openapi_schemas_ops_cases;
#[path = "openapi_schemas_ops_dashboard.rs"]
mod openapi_schemas_ops_dashboard;
#[path = "openapi_schemas_ops_knowledge.rs"]
mod openapi_schemas_ops_knowledge;
#[path = "openapi_schemas_ops_outcomes.rs"]
mod openapi_schemas_ops_outcomes;
#[path = "openapi_schemas_ops_pilot_writeback.rs"]
mod openapi_schemas_ops_pilot_writeback;
#[path = "openapi_schemas_ops_qa_feedback.rs"]
mod openapi_schemas_ops_qa_feedback;
#[path = "openapi_schemas_ops_sampling.rs"]
mod openapi_schemas_ops_sampling;

pub(super) fn ops_schemas() -> Value {
    let mut schemas = Map::new();
    append_schemas(
        &mut schemas,
        openapi_schemas_ops_dashboard::dashboard_schemas(),
    );
    append_schemas(&mut schemas, openapi_schemas_ops_cases::case_schemas());
    append_schemas(
        &mut schemas,
        openapi_schemas_ops_sampling::sampling_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_ops_agent_runs::agent_run_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_ops_knowledge::knowledge_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_ops_agent_investigation::agent_investigation_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_ops_pilot_writeback::pilot_writeback_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_ops_qa_feedback::qa_feedback_schemas(),
    );
    append_schemas(
        &mut schemas,
        openapi_schemas_ops_outcomes::outcome_schemas(),
    );
    Value::Object(schemas)
}

fn append_schemas(target: &mut Map<String, Value>, schemas: Value) {
    let Value::Object(schemas) = schemas else {
        unreachable!("OpenAPI ops schema group must be a JSON object");
    };
    target.extend(schemas);
}
