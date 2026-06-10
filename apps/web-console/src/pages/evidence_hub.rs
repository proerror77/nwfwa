use crate::visual_helpers::*;
use yew::prelude::*;

pub fn evidence_hub_page(on_navigate: Callback<String>) -> Html {
    html! {
        <section class="workflow-hub">
            <div class="dashboard-header">
                <div>
                    <h2>{"Evidence Hub"}</h2>
                    <p>{"Look up the evidence an investigator needs before making a case decision. This keeps context lookup separate from scoring and review actions."}</p>
                </div>
                <span class="status-pill">{"Context lookup"}</span>
            </div>
            {evidence_hub_visual()}
            <div class="workflow-card-grid">
                {workflow_action_card("Evidence Runtime", "Register document packets, chunks, OCR outputs, embedding jobs, and retrieval audit metadata.", "Open runtime", "Evidence Runtime", "strong", &on_navigate)}
                {workflow_action_card("Provider Risk", "Open provider graph signals, suspicious patterns, and network flags.", "Review provider", "Provider Risk", "danger", &on_navigate)}
                {workflow_action_card("Member Profile", "Inspect member-level utilization, policy, and claim history context.", "Review member", "Member Profile", "neutral", &on_navigate)}
                {workflow_action_card("Knowledge Base", "Search confirmed evidence without crossing adjudication boundaries.", "Search evidence", "Knowledge Base", "strong", &on_navigate)}
                {workflow_action_card("Data Sources", "Check dataset lineage, schema mapping, and evaluation inputs.", "Review data", "Data Sources", "success", &on_navigate)}
            </div>
        </section>
    }
}

fn evidence_hub_visual() -> Html {
    html! {
        <section class="panel evidence-visual-shell">
            <div class="evidence-visual-board">
                <div class="evidence-specimen">
                    <div class="specimen-top">
                        <span>{"Document packet"}</span>
                        <strong>{"redacted + traceable"}</strong>
                    </div>
                    <div class="specimen-lines">
                        <i class="wide"></i>
                        <i></i>
                        <i class="short"></i>
                        <i class="wide warning"></i>
                    </div>
                    <div class="specimen-tags">
                        <span>{"checksum"}</span>
                        <span>{"URI"}</span>
                        <span>{"evidence_refs"}</span>
                    </div>
                </div>
                <div class="evidence-pipeline-rail">
                    {evidence_pipeline_node("01", "Register", "document metadata")}
                    {evidence_pipeline_node("02", "OCR", "redacted output")}
                    {evidence_pipeline_node("03", "Chunk", "source spans")}
                    {evidence_pipeline_node("04", "Embed", "job state")}
                    {evidence_pipeline_node("05", "Audit", "retrieval trail")}
                </div>
                <div class="evidence-loop-note">
                    <span>{"Evidence boundary"}</span>
                    <strong>{"LLM sees references, not raw claims text"}</strong>
                    <small>{"The runtime stores provenance, redaction state, retrieval purpose, and actor scope before Agent or QA views consume the packet."}</small>
                </div>
            </div>
        </section>
    }
}

fn evidence_pipeline_node(step: &str, label: &str, caption: &str) -> Html {
    html! {
        <div class="evidence-pipeline-node">
            <span>{step}</span>
            <strong>{label}</strong>
            <small>{caption}</small>
        </div>
    }
}
