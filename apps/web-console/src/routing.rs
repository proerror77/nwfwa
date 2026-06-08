use crate::i18n::tr;
use crate::state::Language;
use yew::prelude::*;

pub(crate) const DEFAULT_MODULE: &str = "Dashboard";

pub(crate) const NAV_SECTIONS: &[(&str, &[&str])] = &[
    (
        "Daily Work",
        &[
            "Dashboard",
            "Leads & Cases",
            "Review Workbench",
            "Bootstrap Ops",
        ],
    ),
    (
        "Control Rooms",
        &[
            "Intake Ops",
            "Discovery Review",
            "Evidence Hub",
            "Governance",
        ],
    ),
];

pub(crate) const ALL_MODULES: &[&str] = &[
    "Intake Ops",
    "Dashboard",
    "Discovery Review",
    "Runtime Scoring",
    "Review Workbench",
    "Bootstrap Ops",
    "Evidence Hub",
    "Provider Model Intake",
    "Evidence Runtime",
    "Rules",
    "Models",
    "Routing Policies",
    "Data Sources",
    "Factor Factory",
    "Leads & Cases",
    "Member Profile",
    "Provider Risk",
    "Medical Review",
    "Audit Sampling",
    "Knowledge Base",
    "Agent Investigator",
    "QA Review",
    "Governance",
];

pub(crate) const CONTRACT_PANELS: &[&str] = &[
    "Management Dashboard",
    "Rule Promotion Gates",
    "Discovery Mode",
    "Candidate Source",
    "Threshold Integrity",
    "Model Governance",
    "Provider Model Intake",
    "Training Jobs",
    "Model Candidates",
    "Offline Training Handoff",
    "Deployment Boundary",
    "Profile Evidence",
    "Candidate Governance",
    "Bootstrap Ops",
    "Historical Replay",
    "Evidence Requests",
    "Label Evidence Handoff",
    "promotion_review_ready",
    "Factor Cards",
    "AUC Gain",
    "Field Governance",
    "Leakage Candidates",
    "SLA Breached",
    "QA Queue",
    "Canonical Evidence",
    "Calibration Signal",
    "Promotion Gate Governance",
    "API Call Records",
    "Guardrail Boundary",
    "Human Gate",
    "Graph Risk",
    "Clinical Signals",
    "Evidence Status",
    "Layer Coverage",
    "Knowledge Base",
    "Graph Evidence Status",
    "Confirmed Evidence",
    "Source Trace",
    "Lineage",
    "Audit Coverage",
    "Canonical Trace Coverage",
    "Canonical Trace",
    "Canonical Trace Only",
    "Input Mode",
];

pub(crate) fn workspace_system_map(
    active: &str,
    on_navigate: Callback<String>,
    language: Language,
) -> Html {
    html! {
        <section class="workspace-system-map" aria-label="FWA platform system map">
            <div class="system-map-rail"></div>
            <div class="system-map-pulse"></div>
            {system_map_stage("Intake", tr(language, "Intake Ops", "进件处理"), tr(language, "TPA packet exceptions", "TPA 案件资料异常"), tr(language, "queue-ready claim", "可进入评分队列"), "Intake Ops", "intake", active, &on_navigate)}
            {system_map_stage("Detect", tr(language, "Scored leads", "已评分线索"), tr(language, "Rules + model + policy", "规则 + 模型 + 路由政策"), tr(language, "human queue", "人工队列"), "Leads & Cases", "detect", active, &on_navigate)}
            {system_map_stage("Review", tr(language, "Human gate", "人工关卡"), tr(language, "Medical + QA", "医疗复核 + QA"), tr(language, "no auto denial", "不由模型自动拒赔"), "Review Workbench", "review", active, &on_navigate)}
            {system_map_stage("Evidence", tr(language, "Case context", "案件上下文"), tr(language, "Member / provider / KB", "会员 / Provider / 知识库"), tr(language, "trace refs", "证据链"), "Evidence Hub", "evidence", active, &on_navigate)}
            {system_map_stage("Govern", tr(language, "Audit trail", "审计轨迹"), tr(language, "Policy + approval", "政策 + 审批"), tr(language, "pilot ready", "试点就绪"), "Governance", "govern", active, &on_navigate)}
            {system_map_stage("Value", tr(language, "Value proof", "价值证明"), tr(language, "Savings evidence", "节省金额证据"), tr(language, "dashboard", "仪表盘"), "Dashboard", "value", active, &on_navigate)}
        </section>
    }
}

pub(crate) fn active_module_from_location() -> String {
    web_sys::window()
        .and_then(|window| window.location().hash().ok())
        .and_then(|hash| module_from_hash(&hash))
        .unwrap_or_else(|| DEFAULT_MODULE.to_string())
}

pub(crate) fn set_module_hash(module: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let slug = module_slug(module);
    if window
        .location()
        .hash()
        .map(|hash| hash == format!("#{slug}"))
        .unwrap_or(false)
    {
        return;
    }
    let _ = window.location().set_hash(slug);
}

pub(crate) fn module_from_hash(hash: &str) -> Option<String> {
    let slug = hash.trim_start_matches('#');
    if slug == "detection-releases" {
        return Some("Discovery Review".to_string());
    }
    ALL_MODULES
        .iter()
        .copied()
        .find(|module| module_slug(module) == slug)
        .map(str::to_string)
}

pub(crate) fn is_known_module(module: &str) -> bool {
    ALL_MODULES.contains(&module)
}

pub(crate) fn module_slug(module: &str) -> &'static str {
    match module {
        "Intake Ops" => "intake-ops",
        "Dashboard" => "dashboard",
        "Discovery Review" => "discovery-review",
        "Runtime Scoring" => "runtime-scoring",
        "Review Workbench" => "review-workbench",
        "Bootstrap Ops" => "bootstrap-ops",
        "Evidence Hub" => "evidence-hub",
        "Provider Model Intake" => "mlops-workspace",
        "Evidence Runtime" => "evidence-runtime",
        "Rules" => "rules",
        "Models" => "models",
        "Routing Policies" => "routing-policies",
        "Data Sources" => "data-sources",
        "Factor Factory" => "factor-factory",
        "Leads & Cases" => "leads-cases",
        "Member Profile" => "member-profile",
        "Provider Risk" => "provider-risk",
        "Medical Review" => "medical-review",
        "Audit Sampling" => "audit-sampling",
        "Knowledge Base" => "knowledge-base",
        "Agent Investigator" => "agent-investigator",
        "QA Review" => "qa-review",
        "Governance" => "governance",
        _ => "dashboard",
    }
}

pub(crate) fn system_map_stage(
    step: &'static str,
    title: &'static str,
    detail: &'static str,
    outcome: &'static str,
    target: &'static str,
    tone: &'static str,
    active: &str,
    on_navigate: &Callback<String>,
) -> Html {
    let target_name = target.to_string();
    let on_navigate = on_navigate.clone();
    let is_active = active == target
        || (target == "Review Workbench" && matches!(active, "Medical Review" | "QA Review"))
        || (target == "Evidence Hub"
            && matches!(
                active,
                "Evidence Runtime"
                    | "Provider Risk"
                    | "Member Profile"
                    | "Knowledge Base"
                    | "Data Sources"
            ));
    html! {
        <button
            class={classes!("system-map-stage", tone, is_active.then_some("active"))}
            onclick={Callback::from(move |_| on_navigate.emit(target_name.clone()))}
        >
            <span class="system-stage-step">{step}</span>
            <span class="system-stage-glyph"></span>
            <strong>{title}</strong>
            <small>{detail}</small>
            <em>{outcome}</em>
        </button>
    }
}

pub(crate) fn module_icon_class(module: &str) -> &'static str {
    match module {
        "Intake Ops" => "icon-inbox",
        "Dashboard" => "icon-dashboard",
        "Discovery Review" => "icon-routing",
        "Runtime Scoring" => "icon-scoring",
        "Review Workbench" => "icon-qa",
        "Bootstrap Ops" => "icon-audit",
        "Evidence Hub" => "icon-knowledge",
        "Provider Model Intake" => "icon-models",
        "Evidence Runtime" => "icon-audit",
        "Rules" => "icon-rules",
        "Models" => "icon-models",
        "Routing Policies" => "icon-routing",
        "Factor Factory" => "icon-factors",
        "Data Sources" => "icon-data",
        "Leads & Cases" => "icon-cases",
        "Member Profile" => "icon-member",
        "Provider Risk" => "icon-provider",
        "Medical Review" => "icon-medical",
        "Audit Sampling" => "icon-audit",
        "Knowledge Base" => "icon-knowledge",
        "Agent Investigator" => "icon-agent",
        "QA Review" => "icon-qa",
        "Governance" => "icon-governance",
        _ => "icon-default",
    }
}
