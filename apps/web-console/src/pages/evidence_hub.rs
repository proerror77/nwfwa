use crate::visual_helpers::*;
use yew::prelude::*;

pub fn evidence_hub_page(on_navigate: Callback<String>) -> Html {
    html! {
        <section class="workflow-hub">
            <div class="dashboard-header">
                <div>
                    <h2>{"证据中心"}</h2>
                    <p>{"为调查员集中查询证据链、成员画像、Provider 图谱和相似案例；这里只提供上下文，不提交分流或赔付结论。"}</p>
                </div>
                <span class="status-pill">{"上下文查询"}</span>
            </div>
            {evidence_hub_visual()}
            <div class="workflow-card-grid">
                {workflow_action_card("证据运行时", "查看文件包、OCR、chunk、embedding 和检索审计元数据。", "查看证据链", "Evidence Runtime", "strong", &on_navigate)}
                {workflow_action_card("Provider 风险", "查看供应商图谱信号、异常模式和网络关联。", "查看 Provider", "Provider Risk", "danger", &on_navigate)}
                {workflow_action_card("成员画像", "查看成员级使用历史、保单和理赔上下文。", "查看成员", "Member Profile", "neutral", &on_navigate)}
                {workflow_action_card("知识库", "检索已确认案例和相似证据，不跨越裁决边界。", "检索案例", "Knowledge Base", "strong", &on_navigate)}
                {workflow_action_card("数据来源", "检查数据集血缘、字段映射和模型评估输入。", "查看数据", "Data Sources", "success", &on_navigate)}
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
                        <span>{"证据包"}</span>
                        <strong>{"脱敏 + 可追踪"}</strong>
                    </div>
                    <div class="specimen-lines">
                        <i class="wide"></i>
                        <i></i>
                        <i class="short"></i>
                        <i class="wide warning"></i>
                    </div>
                    <div class="specimen-tags">
                        <span>{"校验和"}</span>
                        <span>{"URI"}</span>
                        <span>{"evidence_refs"}</span>
                    </div>
                </div>
                <div class="evidence-pipeline-rail">
                    {evidence_pipeline_node("01", "登记", "document metadata")}
                    {evidence_pipeline_node("02", "OCR", "redacted output")}
                    {evidence_pipeline_node("03", "切片", "source spans")}
                    {evidence_pipeline_node("04", "向量化", "job state")}
                    {evidence_pipeline_node("05", "审计", "retrieval trail")}
                </div>
                <div class="evidence-loop-note">
                    <span>{"证据边界"}</span>
                    <strong>{"AI 只消费引用，不直接暴露原始理赔文本"}</strong>
                    <small>{"运行时在 Agent 或 QA 查看证据包前记录来源、脱敏状态、检索目的和 actor scope。"}</small>
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
