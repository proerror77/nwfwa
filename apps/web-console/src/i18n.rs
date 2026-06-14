use crate::state::Language;
use wasm_bindgen::prelude::wasm_bindgen;

// Translation table is compiled in from assets/i18n/zh-CN.json.
// JS receives it once via init_translations() before any DOM language switch.
const ZH_CN_JSON: &str = include_str!("../../../assets/i18n/zh-CN.json");

#[wasm_bindgen(inline_js = r#"
let translations = new Map();
const translatedValues = new Set();

export function initTranslations(jsonStr) {
  const obj = JSON.parse(jsonStr);
  translations = new Map(Object.entries(obj));
  translations.forEach(v => translatedValues.add(v));
}

const originalText = new WeakMap();
const originalAttrs = new WeakMap();
let currentLanguage = "en";
let observer = null;
let applying = false;

function withOriginalSpacing(source, translated) {
  const prefix = source.match(/^\s*/)?.[0] ?? "";
  const suffix = source.match(/\s*$/)?.[0] ?? "";
  return `${prefix}${translated}${suffix}`;
}

function sourceForTextNode(node) {
  const value = node.nodeValue ?? "";
  const trimmed = value.trim();
  let source = originalText.get(node);
  if (!source || (trimmed !== source.trim() && !translatedValues.has(trimmed))) {
    source = value;
    originalText.set(node, source);
  }
  return source;
}

function translateTextNode(node, language) {
  const source = sourceForTextNode(node);
  const key = source.trim();
  if (!key) return;
  if (language === "zh-CN") {
    const translated = translations.get(key);
    if (translated) {
      const nextValue = withOriginalSpacing(source, translated);
      if (node.nodeValue !== nextValue) node.nodeValue = nextValue;
    }
  } else {
    if (node.nodeValue !== source) node.nodeValue = source;
  }
}

function rememberAttr(element, attr) {
  const value = element.getAttribute(attr);
  if (!value) return null;
  let attrs = originalAttrs.get(element);
  if (!attrs) {
    attrs = {};
    originalAttrs.set(element, attrs);
  }
  const trimmed = value.trim();
  if (!attrs[attr] || (trimmed !== attrs[attr].trim() && !translatedValues.has(trimmed))) {
    attrs[attr] = value;
  }
  return attrs[attr];
}

function translateAttr(element, attr, language) {
  const source = rememberAttr(element, attr);
  if (!source) return;
  if (language === "zh-CN") {
    const translated = translations.get(source.trim());
    if (translated && element.getAttribute(attr) !== translated) element.setAttribute(attr, translated);
  } else {
    if (element.getAttribute(attr) !== source) element.setAttribute(attr, source);
  }
}

function shouldSkipText(parent) {
  if (!parent) return true;
  const tagName = parent.nodeName;
  return tagName === "SCRIPT" || tagName === "STYLE" || tagName === "PRE" || tagName === "CODE";
}

function translateRoot(root, language) {
  applying = true;
  try {
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    const nodes = [];
    while (walker.nextNode()) nodes.push(walker.currentNode);
    for (const node of nodes) {
      if (!shouldSkipText(node.parentElement)) translateTextNode(node, language);
    }
    for (const element of root.querySelectorAll("[placeholder],[aria-label],[title]")) {
      translateAttr(element, "placeholder", language);
      translateAttr(element, "aria-label", language);
      translateAttr(element, "title", language);
    }
  } finally {
    applying = false;
  }
}

export function applyDocumentLanguage(language) {
  currentLanguage = language;
  document.documentElement.lang = language;
  const root = document.querySelector(".app");
  if (!root) return;
  translateRoot(root, language);
  if (observer) return;
  observer = new MutationObserver(() => {
    if (applying || currentLanguage !== "zh-CN") return;
    const appRoot = document.querySelector(".app");
    if (appRoot) translateRoot(appRoot, currentLanguage);
  });
  observer.observe(root, {
    childList: true,
    subtree: true,
    characterData: true,
    attributes: true,
    attributeFilter: ["placeholder", "aria-label", "title"]
  });
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = initTranslations)]
    fn init_translations(json: &str);

    #[wasm_bindgen(js_name = applyDocumentLanguage)]
    pub(crate) fn apply_document_language(language: &str);
}

/// Call once at startup to push the compiled-in JSON into the JS translation Map.
pub(crate) fn setup_translations() {
    init_translations(ZH_CN_JSON);
}

pub(crate) fn tr(language: Language, en: &'static str, zh: &'static str) -> &'static str {
    match language {
        Language::En => en,
        Language::Zh => zh,
    }
}

pub(crate) fn brand_description(language: Language) -> &'static str {
    tr(
        language,
        "Operations desk for claim scoring, case triage, reviewer queues, and pilot governance.",
        "用于理赔评分、案件分流、审核队列和试点治理的风控运营台。",
    )
}

pub(crate) fn section_label(section: &str, language: Language) -> &'static str {
    match section {
        "Daily Work" => tr(language, "Daily Work", "日常作业"),
        "Control Rooms" => tr(language, "Control Rooms", "控制室"),
        "Daily Ops" => tr(language, "Daily Ops", "日常运营"),
        "Intake & Scoring" => tr(language, "Intake & Scoring", "进件与评分"),
        "Investigation" => tr(language, "Investigation", "调查"),
        "Governance & Tuning" => tr(language, "Governance & Tuning", "治理与调优"),
        _ => "Section",
    }
}

pub(crate) fn module_label(module: &str, language: Language) -> &'static str {
    match module {
        "Intake Ops" => tr(language, "Intake Ops", "进件处理"),
        "Dashboard" => tr(language, "Dashboard", "运营仪表盘"),
        "Discovery Review" => tr(language, "Discovery Review", "发现评审"),
        "Runtime Scoring" => tr(language, "Runtime Scoring", "实时评分"),
        "Review Workbench" => tr(language, "Review Workbench", "复核工作台"),
        "Bootstrap Ops" => tr(language, "Training Label Handoff", "训练标签交付"),
        "Evidence Hub" => tr(language, "Evidence Hub", "证据中心"),
        "Provider Model Intake" => tr(language, "Provider Model Intake", "Provider 模型接入"),
        "Evidence Runtime" => tr(language, "Evidence Runtime", "证据运行时"),
        "Rules" => tr(language, "Rules", "规则"),
        "Models" => tr(language, "Models", "模型"),
        "Routing Policies" => tr(language, "Review Routing Policies", "审核分流策略"),
        "Factor Factory" => tr(language, "Factor Factory", "因子工厂"),
        "Data Sources" => tr(language, "Data Sources", "数据源"),
        "Leads & Cases" => tr(language, "Leads & Cases", "线索与案件"),
        "Member Profile" => tr(language, "Member Profile", "会员画像"),
        "Provider Risk" => tr(language, "Provider Risk", "Provider 风险"),
        "Medical Review" => tr(language, "Medical Review", "医疗复核"),
        "Audit Sampling" => tr(language, "Audit Sampling", "审计抽样"),
        "Knowledge Base" => tr(language, "Knowledge Base", "知识库"),
        "Agent Investigator" => tr(language, "Agent Investigator", "辅助调查"),
        "QA Review" => tr(language, "QA Review", "QA 复核"),
        "Governance" => tr(language, "Governance", "治理"),
        _ => "Module",
    }
}

pub(crate) fn module_context(module: &str, language: Language) -> &'static str {
    match module {
        "Intake Ops" => tr(
            language,
            "Resolve inbound TPA packet exceptions before claims enter risk and review queues.",
            "先处理 TPA 进件资料异常，再让案件进入评分和复核队列。",
        ),
        "Dashboard" => tr(
            language,
            "Choose the next operational action from live risk and review queues.",
            "从实时风险与复核队列中选择下一步运营动作。",
        ),
        "Discovery Review" => tr(
            language,
            "Review ML-discovered rules and provider model candidates before shadow, release, or rejection.",
            "在影子运行、发布或拒绝前，审查模型发现的规则和 Provider 模型候选。",
        ),
        "Runtime Scoring" => tr(
            language,
            "Validate the scoring API contract, routing policy, evidence refs, and audit IDs.",
            "验证评分 API 契约、路由策略、证据引用和审计 ID。",
        ),
        "Review Workbench" => tr(
            language,
            "Resolve clinical and QA review queues from one place.",
            "在一个入口处理医疗复核和 QA 队列。",
        ),
        "Bootstrap Ops" => tr(
            language,
            "Prepare evidence-backed labels for the independent training platform handoff.",
            "为独立训练平台准备有证据支撑的标签交付包。",
        ),
        "Evidence Hub" => tr(
            language,
            "Open member, provider, knowledge, and dataset context from one evidence hub.",
            "在证据中心查看会员、Provider、知识库和数据集上下文。",
        ),
        "Provider Model Intake" => tr(
            language,
            "Review provider-trained model candidates and decide release, shadow, or rollback.",
            "审查 Provider 训练的模型候选，并决定发布、影子运行或回滚。",
        ),
        "Evidence Runtime" => tr(
            language,
            "Register document, OCR, chunk, embedding, and retrieval metadata with audit trace.",
            "登记文件、OCR、切块、embedding 和检索元数据，并保留审计轨迹。",
        ),
        "Rules" => tr(
            language,
            "Review offline-mined rule candidates before they enter the active rule library.",
            "审查离线挖掘出的规则候选，再决定是否进入启用规则库。",
        ),
        "Models" => tr(
            language,
            "Review model readiness, thresholds, and production evidence.",
            "审查模型就绪度、阈值和生产证据。",
        ),
        "Routing Policies" => tr(
            language,
            "Configure when risk bands route claims to STP, QA sampling, or manual review.",
            "设置不同风险等级进入自动通过、抽样复核或人工审核。",
        ),
        "Factor Factory" => tr(
            language,
            "Govern feature readiness, ownership, and online availability.",
            "治理特征就绪度、负责人和在线可用性。",
        ),
        "Data Sources" => tr(
            language,
            "Control datasets, schema mappings, and model evaluation lineage.",
            "管理数据集、字段映射和模型评估血缘。",
        ),
        "Leads & Cases" => tr(
            language,
            "Move scored leads into investigation and case workflows.",
            "把已评分线索推进调查和案件流程。",
        ),
        "Member Profile" => tr(
            language,
            "Inspect member-level risk evidence and utilization context.",
            "查看会员层级的风险证据和使用情况。",
        ),
        "Provider Risk" => tr(
            language,
            "Review provider graph signals and suspicious practice patterns.",
            "查看 Provider 图谱信号和可疑执业模式。",
        ),
        "Medical Review" => tr(
            language,
            "Route clinical evidence to human review with traceable outcomes.",
            "把临床证据送入人工医疗复核并保留可追溯结论。",
        ),
        "Audit Sampling" => tr(
            language,
            "Sample decisions for QA, compliance, and model governance.",
            "抽样检查决策，用于 QA、合规和模型治理。",
        ),
        "Knowledge Base" => tr(
            language,
            "Search confirmed evidence without crossing adjudication boundaries.",
            "搜索已确认案例证据，但不越过理赔裁决边界。",
        ),
        "Agent Investigator" => tr(
            language,
            "Run assistive investigation with human decision gates.",
            "运行辅助调查，但关键决策保留人工关卡。",
        ),
        "QA Review" => tr(
            language,
            "Close feedback loops for findings, calibration, and reviewer quality.",
            "闭环发现、校准和审核质量反馈。",
        ),
        "Governance" => tr(
            language,
            "Audit API calls, agent boundaries, and evidence trace coverage.",
            "审计 API 调用、Agent 边界和证据链覆盖率。",
        ),
        _ => tr(
            language,
            "Operate the FWA pilot workspace.",
            "操作 FWA 试点工作台。",
        ),
    }
}

pub(crate) fn module_description(module: &str, language: Language) -> &'static str {
    match module {
        "Intake Ops" => tr(language, "intake exceptions", "进件异常"),
        "Dashboard" => tr(language, "next action", "下一步动作"),
        "Discovery Review" => tr(language, "ML review gate", "模型评审关卡"),
        "Runtime Scoring" => tr(language, "contract check", "契约检查"),
        "Review Workbench" => tr(language, "medical + QA", "医疗 + QA"),
        "Bootstrap Ops" => tr(language, "labels + evidence", "标签 + 证据"),
        "Evidence Hub" => tr(language, "context lookup", "上下文查询"),
        "Provider Model Intake" => tr(language, "model candidate release", "模型候选发布"),
        "Evidence Runtime" => tr(language, "document evidence", "文件证据"),
        "Rules" => tr(language, "rule candidate release", "规则候选发布"),
        "Models" => tr(language, "threshold evidence", "阈值证据"),
        "Routing Policies" => tr(language, "review routing", "审核分流"),
        "Factor Factory" => tr(language, "feature readiness", "特征就绪"),
        "Data Sources" => tr(language, "catalog & lineage", "目录与血缘"),
        "Leads & Cases" => tr(language, "investigation queue", "调查队列"),
        "Member Profile" => tr(language, "member context", "会员上下文"),
        "Provider Risk" => tr(language, "provider signals", "Provider 信号"),
        "Medical Review" => tr(language, "clinical review", "临床复核"),
        "Audit Sampling" => tr(language, "sample governance", "抽样治理"),
        "Knowledge Base" => tr(language, "confirmed evidence", "已确认案例"),
        "Agent Investigator" => tr(language, "assistive agent", "辅助调查"),
        "QA Review" => tr(language, "feedback closure", "反馈闭环"),
        "Governance" => tr(language, "audit boundary", "审计边界"),
        _ => tr(language, "module", "模块"),
    }
}
