use crate::api::*;
use crate::types::*;
use crate::constants::*;
use crate::state::{use_api_key, ApiState};
use crate::formatting::*;
use crate::ui_helpers::*;
use crate::visual_helpers::*;
use crate::case_helpers::*;
use crate::rule_helpers::*;
use crate::rule_ui_helpers::*;
use crate::inbox_helpers::*;
use crate::payload_helpers::*;
use crate::data_helpers::*;
use crate::data_lineage_helpers::*;
use crate::medical_review_helpers::*;
use crate::model_ui_helpers::*;
use crate::runtime_helpers::*;
use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[function_component(DataSourcesPage)]
pub fn data_sources_page() -> Html {
    let api_key = use_api_key();
    let snapshot_state = use_state(|| ApiState::<DataSourcesSnapshot>::Idle);

    let load_sources = {
        let api_key = api_key.clone();
        let snapshot_state = snapshot_state.clone();
        Callback::from(move |_| {
            let api_key = (*api_key).clone();
            let snapshot_state = snapshot_state.clone();
            snapshot_state.set(ApiState::Loading);
            spawn_local(async move {
                snapshot_state.set(match get_data_sources_snapshot(api_key).await {
                    Ok(snapshot) => ApiState::Ready(snapshot),
                    Err(error) => ApiState::Failed(error),
                });
            });
        })
    };

    let refresh = {
        let load_sources = load_sources.clone();
        Callback::from(move |_| load_sources.emit(()))
    };

    {
        let load_sources = load_sources.clone();
        use_effect_with((), move |_| {
            load_sources.emit(());
            || ()
        });
    }

    html! {
        <section class="module-status">
            <div class="dashboard-header">
                <div>
                    <h2>{"Data Sources"}</h2>
                    <p>{"Inspect parquet dataset catalog, data health, schema coverage, field mappings, and model evaluation lineage for governed feature and model operations."}</p>
                </div>
                <span class="status-pill">{"Data & Metric Foundation"}</span>
            </div>

            <section class="panel">
                <h3>{"Data Source Control"}</h3>
                <p class="empty">{"Using the configured data-governance workspace for catalog, schema, and evaluation lineage."}</p>
                <div class="button-row">
                    <button onclick={refresh} disabled={matches!(&*snapshot_state, ApiState::Loading)}>
                        {if matches!(&*snapshot_state, ApiState::Loading) { "Refreshing..." } else { "Refresh data sources" }}
                    </button>
                </div>
            </section>

            <DataSourcesView state={(*snapshot_state).clone()} />
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct DataSourcesProps {
    state: ApiState<DataSourcesSnapshot>,
}

#[function_component(DataSourcesView)]
fn data_sources_view(props: &DataSourcesProps) -> Html {
    html! {
        <>
            {match &props.state {
                ApiState::Idle => html! { <section class="panel"><p class="empty">{"Load data sources to inspect catalog and lineage."}</p></section> },
                ApiState::Loading => html! { <section class="panel"><p>{"Loading data source catalog..."}</p></section> },
                ApiState::Failed(error) => html! { <section class="panel"><p class="error">{error}</p></section> },
                ApiState::Ready(snapshot) => html! {
                    <>
                        <section class="panel data-command-center">
                            <div class="section-header">
                                <div>
                                    <h3>{"Data Foundation Control"}</h3>
                                    <p>{"External claim, policy, provider, and medical datasets must stay traceable before rules, models, and agents can rely on them."}</p>
                                </div>
                            </div>
                            <div class="ops-stat-strip">
                                <div><span>{"Datasets"}</span><strong>{snapshot.datasets.len()}</strong><small>{"registered sources"}</small></div>
                                <div><span>{"Rows"}</span><strong>{total_dataset_rows(&snapshot.datasets)}</strong><small>{"available records"}</small></div>
                                <div><span>{"Fields"}</span><strong>{total_schema_fields(&snapshot.datasets)}</strong><small>{"profiled columns"}</small></div>
                                <div><span>{"Mappings"}</span><strong>{total_field_mappings(&snapshot.datasets)}</strong><small>{"canonical links"}</small></div>
                                <div><span>{"Evaluations"}</span><strong>{snapshot.evaluations.len()}</strong><small>{"model runs"}</small></div>
                            </div>
                        </section>

                        {data_lineage_cockpit(snapshot)}

                        <section class="panel result-stack">
                            <div class="section-header">
                                <div>
                                    <h3>{"Dataset Catalog"}</h3>
                                    <p>{"Which governed datasets can feed scoring, feature creation, and medical review workflows."}</p>
                                </div>
                            </div>
                            if snapshot.datasets.is_empty() {
                                <p class="empty">{"No datasets registered."}</p>
                            } else {
                                <div class="ops-table dataset-catalog-table">
                                    <div class="ops-table-head">
                                        <span>{"Dataset"}</span>
                                        <span>{"Domain"}</span>
                                        <span>{"Rows"}</span>
                                        <span>{"Grain"}</span>
                                        <span>{"Status"}</span>
                                    </div>
                                    {for snapshot.datasets.iter().map(|dataset| html! {
                                        <div class="ops-table-row">
                                            <div class="primary-cell">
                                                <strong>{&dataset.display_name}</strong>
                                                <span>{format!("{}:{} / {}", dataset.dataset_key, dataset.dataset_version, dataset.storage_format)}</span>
                                            </div>
                                            <span>{&dataset.business_domain}</span>
                                            <strong>{dataset.row_count}</strong>
                                            <span>{&dataset.sample_grain}</span>
                                            <span class={classes!("status-token", status_tone(&dataset.status))}>{&dataset.status}</span>
                                            <small class="row-detail">{format!("source {} / label {} / keys {} / manifest {}", dataset.source_key, empty_label(&dataset.label_column), refs_label(&dataset.entity_keys), dataset.manifest_uri)}</small>
                                        </div>
                                    })}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <div class="section-header">
                                <div>
                                    <h3>{"Dataset Health"}</h3>
                                    <p>{"Operational readiness signals used before a dataset is trusted for scoring or training."}</p>
                                </div>
                            </div>
                            if snapshot.health.is_empty() {
                                <p class="empty">{"No dataset health records returned."}</p>
                            } else {
                                <div class="health-grid">
                                    {for snapshot.health.iter().map(|health| html! {
                                        <div class="health-card">
                                            <div>
                                                <strong>{format!("{}:{}", health.dataset_key, health.dataset_version)}</strong>
                                                <span class={classes!("status-token", status_tone(&health.data_quality_status))}>{format!("{} / {:.2}", health.data_quality_status, health.data_quality_score)}</span>
                                            </div>
                                            <dl>
                                                <div><dt>{"Fields"}</dt><dd>{health.field_count}</dd></div>
                                                <div><dt>{"Labels"}</dt><dd>{health.label_count}</dd></div>
                                                <div><dt>{"Keys"}</dt><dd>{health.entity_key_count}</dd></div>
                                                <div><dt>{"Online"}</dt><dd>{health.online_ready_count}</dd></div>
                                                <div><dt>{"Issues"}</dt><dd>{health.issue_count}</dd></div>
                                                <div><dt>{"Missing"}</dt><dd>{health.high_missing_count}</dd></div>
                                            </dl>
                                            <small>{format!("unstable {} / unowned {}", health.unstable_field_count, health.unowned_field_count)}</small>
                                        </div>
                                    })}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <div class="section-header">
                                <div>
                                    <h3>{"Split And Schema Coverage"}</h3>
                                    <p>{"Train/validation/test splits and schema fields that determine what can become features, labels, or review evidence."}</p>
                                </div>
                            </div>
                            if snapshot.datasets.is_empty() {
                                <p class="empty">{"No split or schema coverage available."}</p>
                            } else {
                                <div class="dataset-workbench-list">
                                    {for snapshot.datasets.iter().map(|dataset| html! {
                                        <article class="dataset-workbench">
                                            <div class="workbench-title">
                                                <div>
                                                    <strong>{format!("{}:{}", dataset.dataset_key, dataset.dataset_version)}</strong>
                                                    <span>{format!("schema hash: {}", dataset.schema_hash)}</span>
                                                </div>
                                                <span class="status-token neutral">{format!("{} fields", dataset.fields.len())}</span>
                                            </div>
                                            <div class="workbench-grid">
                                                <div>
                                                    <h4>{"Splits"}</h4>
                                                    if dataset.splits.is_empty() {
                                                        <p class="empty">{"No split records."}</p>
                                                    } else {
                                                        <div class="split-list">
                                                            {for dataset.splits.iter().map(|split| html! {
                                                                <div class="split-row">
                                                                    <div>
                                                                        <strong>{&split.split_name}</strong>
                                                                        <span>{&split.data_uri}</span>
                                                                    </div>
                                                                    <div><span>{"Rows"}</span><strong>{split.row_count}</strong></div>
                                                                    <div><span>{"Labels"}</span><strong>{format!("+{} / -{}", optional_u64(split.positive_count), optional_u64(split.negative_count))}</strong></div>
                                                                    <small>{format!("distribution: {}", payload_keys_label(&split.label_distribution_json))}</small>
                                                                </div>
                                                            })}
                                                        </div>
                                                    }
                                                </div>
                                                <div>
                                                    <h4>{"Schema Fields"}</h4>
                                                    <div class="field-table">
                                                        <div class="field-table-head">
                                                            <span>{"Field"}</span>
                                                            <span>{"Type / Role"}</span>
                                                            <span>{"Nullability"}</span>
                                                            <span>{"Profile"}</span>
                                                        </div>
                                                        {for dataset.fields.iter().map(|field| html! {
                                                            <div class="field-row">
                                                                <div class="primary-cell">
                                                                    <strong>{&field.field_name}</strong>
                                                                    <span>{empty_label(&field.description)}</span>
                                                                </div>
                                                                <div class="chip-row">
                                                                    <span class="type-chip">{&field.logical_type}</span>
                                                                    <span class="role-chip">{&field.semantic_role}</span>
                                                                </div>
                                                                <span class={classes!("status-token", if field.nullable { "neutral" } else { "strong" })}>
                                                                    {if field.nullable { "nullable" } else { "required" }}
                                                                </span>
                                                                <details class="data-source-detail field-profile-detail">
                                                                    <summary>{payload_signal_count_label(&field.profile_json, "profile signals")}</summary>
                                                                    <small>{payload_keys_label(&field.profile_json)}</small>
                                                                </details>
                                                            </div>
                                                        })}
                                                    </div>
                                                </div>
                                            </div>
                                        </article>
                                    })}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <div class="section-header">
                                <div>
                                    <h3>{"Field Mapping Lineage"}</h3>
                                    <p>{"How external fields become canonical claims, policy, provider, member, or feature entities."}</p>
                                </div>
                            </div>
                            if !snapshot.datasets.iter().any(|dataset| !dataset.mappings.is_empty()) {
                                <p class="empty">{"No field mappings registered."}</p>
                            } else {
                                <div class="lineage-list">
                                    {for snapshot.datasets.iter().flat_map(|dataset| {
                                        dataset.mappings.iter().map(move |mapping| (dataset, mapping))
                                    }).map(|(dataset, mapping)| html! {
                                        <div class="lineage-row">
                                            <div class="lineage-flow">
                                                <strong>{&mapping.external_field}</strong>
                                                <span>{"->"}</span>
                                                <strong>{&mapping.canonical_target}</strong>
                                            </div>
                                                <span>{mapping.feature_name.as_deref().unwrap_or("no feature")}</span>
                                                <span class={classes!("status-token", status_tone(&mapping.status))}>{&mapping.status}</span>
                                            <details class="data-source-detail">
                                                <summary>{format!("{}:{} / {}", dataset.dataset_key, dataset.dataset_version, mapping.transform_kind)}</summary>
                                                <small>{format!("transform {}", payload_keys_label(&mapping.transform_json))}</small>
                                            </details>
                                        </div>
                                    })}
                                </div>
                            }
                        </section>

                        <section class="panel result-stack">
                            <div class="section-header">
                                <div>
                                    <h3>{"Model Evaluation Lineage"}</h3>
                                    <p>{"Which dataset version and data quality state produced each model evaluation result."}</p>
                                </div>
                            </div>
                            if snapshot.evaluations.is_empty() {
                                <p class="empty">{"No model evaluations registered."}</p>
                            } else {
                                <div class="ops-table evaluation-table">
                                    <div class="ops-table-head">
                                        <span>{"Model"}</span>
                                        <span>{"Run"}</span>
                                        <span>{"AUC"}</span>
                                        <span>{"Precision"}</span>
                                        <span>{"Recall"}</span>
                                        <span>{"Data Quality"}</span>
                                    </div>
                                    {for snapshot.evaluations.iter().map(|evaluation| {
                                        let lineage = lineage_for(&snapshot.lineage, &evaluation.evaluation_run_id);
                                        html! {
                                            <div class="ops-table-row">
                                                <div class="primary-cell">
                                                    <strong>{format!("{} / {}", evaluation.model_key, evaluation.model_version)}</strong>
                                                    <span>{format!("dataset {}", evaluation.model_dataset_id)}</span>
                                                </div>
                                                <span>{format!("{} / {}", evaluation.evaluation_run_id, evaluation.scheme_family)}</span>
                                                <strong>{optional_metric(&evaluation.auc)}</strong>
                                                <strong>{optional_metric(&evaluation.precision)}</strong>
                                                <strong>{optional_metric(&evaluation.recall)}</strong>
                                                <span>{lineage_data_quality_label(lineage)}</span>
                                                <details class="row-detail data-source-detail evaluation-evidence-detail">
                                                    <summary>{format!("Evaluation evidence detail: f1 {} / threshold {}", optional_metric(&evaluation.f1), optional_metric(&evaluation.threshold))}</summary>
                                                    <div class="data-source-detail-grid">
                                                        <small>{format!("source {}", lineage_source_label(lineage))}</small>
                                                        <small>{format!("metrics {}", payload_signal_count_label(&evaluation.metrics_json, "metric fields"))}</small>
                                                        <small>{format!("confusion {}", payload_signal_count_label(&evaluation.confusion_matrix_json, "confusion fields"))}</small>
                                                        <small>{format!("feature importance {}", evaluation.feature_importance_uri.as_deref().unwrap_or("none"))}</small>
                                                        <small>{format!("permutation importance {}", evaluation.permutation_importance_uri.as_deref().unwrap_or("none"))}</small>
                                                    </div>
                                                </details>
                                            </div>
                                        }
                                    })}
                                </div>
                            }
                        </section>
                    </>
                },
            }}
        </>
    }
}
