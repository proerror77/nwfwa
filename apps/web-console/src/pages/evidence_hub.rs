use crate::i18n::tr;
use crate::state::Language;
use crate::visual_helpers::*;
use yew::prelude::*;

pub fn evidence_hub_page(on_navigate: Callback<String>) -> Html {
    evidence_hub_page_with_language(on_navigate, Language::Zh)
}

pub fn evidence_hub_page_with_language(on_navigate: Callback<String>, language: Language) -> Html {
    html! {
        <section class="workflow-hub">
            <div class="dashboard-header">
                <div>
                    <h2>{tr(language, "Evidence Center", "证据中心")}</h2>
                    <p>{tr(language, "Central context lookup for evidence chain, member profile, provider graph, and similar cases. This page provides context only and does not submit triage or payment decisions.", "为调查员集中查询证据链、成员画像、Provider 图谱和相似案例；这里只提供上下文，不提交分流或赔付结论。")}</p>
                </div>
                <span class="status-pill">{tr(language, "Context lookup", "上下文查询")}</span>
            </div>
            {evidence_hub_visual(language)}
            <div class="workflow-card-grid">
                {workflow_action_card(tr(language, "Evidence Runtime", "证据运行时"), tr(language, "Inspect document packets, OCR, chunks, embeddings, and retrieval audit metadata.", "查看文件包、OCR、chunk、embedding 和检索审计元数据。"), tr(language, "Open evidence chain", "查看证据链"), "Evidence Runtime", "strong", &on_navigate)}
                {workflow_action_card(tr(language, "Provider Risk", "Provider 风险"), tr(language, "Review provider graph signals, anomaly patterns, and network associations.", "查看供应商图谱信号、异常模式和网络关联。"), tr(language, "Open Provider", "查看 Provider"), "Provider Risk", "danger", &on_navigate)}
                {workflow_action_card(tr(language, "Member Profile", "成员画像"), tr(language, "Review member-level utilization history, policies, and claim context.", "查看成员级使用历史、保单和理赔上下文。"), tr(language, "Open member", "查看成员"), "Member Profile", "neutral", &on_navigate)}
                {workflow_action_card(tr(language, "Knowledge Base", "知识库"), tr(language, "Search confirmed cases and similar evidence without crossing adjudication boundaries.", "检索已确认案例和相似证据，不跨越裁决边界。"), tr(language, "Search cases", "检索案例"), "Knowledge Base", "strong", &on_navigate)}
                {workflow_action_card(tr(language, "Data Sources", "数据来源"), tr(language, "Inspect dataset lineage, field mappings, and model evaluation inputs.", "检查数据集血缘、字段映射和模型评估输入。"), tr(language, "Open data", "查看数据"), "Data Sources", "success", &on_navigate)}
            </div>
        </section>
    }
}

fn evidence_hub_visual(language: Language) -> Html {
    html! {
        <section class="panel evidence-visual-shell">
            <div class="evidence-visual-board">
                <div class="evidence-specimen">
                    <div class="specimen-top">
                        <span>{tr(language, "Evidence packet", "证据包")}</span>
                        <strong>{tr(language, "Redacted + traceable", "脱敏 + 可追踪")}</strong>
                    </div>
                    <div class="specimen-lines">
                        <i class="wide"></i>
                        <i></i>
                        <i class="short"></i>
                        <i class="wide warning"></i>
                    </div>
                    <div class="specimen-tags">
                        <span>{tr(language, "Checksum", "校验和")}</span>
                        <span>{"URI"}</span>
                        <span>{"evidence_refs"}</span>
                    </div>
                </div>
                <div class="evidence-pipeline-rail">
                    {evidence_pipeline_node("01", tr(language, "Register", "登记"), "document metadata")}
                    {evidence_pipeline_node("02", "OCR", "redacted output")}
                    {evidence_pipeline_node("03", tr(language, "Chunk", "切片"), "source spans")}
                    {evidence_pipeline_node("04", tr(language, "Embed", "向量化"), "job state")}
                    {evidence_pipeline_node("05", tr(language, "Audit", "审计"), "retrieval trail")}
                </div>
                <div class="evidence-loop-note">
                    <span>{tr(language, "Evidence boundary", "证据边界")}</span>
                    <strong>{tr(language, "AI consumes references only and does not expose raw claim text directly.", "AI 只消费引用，不直接暴露原始理赔文本")}</strong>
                    <small>{tr(language, "The runtime records source, redaction state, retrieval purpose, and actor scope before Agent or QA views evidence packets.", "运行时在 Agent 或 QA 查看证据包前记录来源、脱敏状态、检索目的和 actor scope。")}</small>
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
