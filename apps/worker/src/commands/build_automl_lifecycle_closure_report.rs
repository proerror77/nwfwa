use super::flags::{take_flag_value, take_repeated_flag_value};

pub fn run(mut args: Vec<String>) -> anyhow::Result<()> {
    let demo_index = take_flag_value(&mut args, "--demo-index")?;
    let candidate_ranking = take_flag_value(&mut args, "--candidate-ranking")?;
    let artifact_evaluation_reports =
        take_repeated_flag_value(&mut args, "--artifact-evaluation-report")?;
    let rule_backtest_report = take_flag_value(&mut args, "--rule-backtest-report")?;
    let provider_clustering_report = take_flag_value(&mut args, "--provider-clustering-report")?;
    let provider_graph_report = take_flag_value(&mut args, "--provider-graph-report")?;
    let claim_entity_clustering_report =
        take_flag_value(&mut args, "--claim-entity-clustering-report")?;
    let mlops_monitoring_report = take_flag_value(&mut args, "--mlops-monitoring-report")?;
    let mlops_scheduler_execution_report =
        take_flag_value(&mut args, "--mlops-scheduler-execution-report")?;
    let mlops_monitoring_cycle_report =
        take_flag_value(&mut args, "--mlops-monitoring-cycle-report")?;
    let model_promotion_orchestration_report =
        take_flag_value(&mut args, "--model-promotion-orchestration-report")?;
    let output_dir = take_flag_value(&mut args, "--output-dir")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments: {}", args.join(" "));
    }
    let report = crate::build_automl_lifecycle_closure_report(
        &demo_index,
        &candidate_ranking,
        &artifact_evaluation_reports,
        &rule_backtest_report,
        &provider_clustering_report,
        &provider_graph_report,
        &claim_entity_clustering_report,
        &mlops_monitoring_report,
        &mlops_scheduler_execution_report,
        &mlops_monitoring_cycle_report,
        &model_promotion_orchestration_report,
        output_dir,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
