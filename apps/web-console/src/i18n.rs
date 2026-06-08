use crate::state::Language;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(inline_js = r#"
const translations = new Map([
  ["FWA Platform", "FWA 平台"],
  ["Real-time operations", "实时运营"],
  ["live", "在线"],
  ["Pilot Ops", "试点运营"],
  ["Dashboard", "运营仪表盘"],
  ["Pilot Operations", "试点运营"],
  ["Watch the operating queue, risk value, review load, and governance health without exposing low-frequency integration tools.", "查看运营队列、风险价值、复核负载和治理健康度，同时隐藏低频集成工具。"],
  ["Dashboard Source", "仪表盘数据源"],
  ["Using the configured pilot operations principal for queue, value, review-load, and governance signals.", "使用已配置的试点运营身份读取队列、价值、复核负载和治理信号。"],
  ["Refresh dashboard", "刷新仪表盘"],
  ["Refreshing...", "刷新中..."],
  ["Load the dashboard to inspect operational value and governance coverage.", "加载仪表盘以查看运营价值和治理覆盖。"],
  ["Loading dashboard summary...", "正在加载仪表盘摘要..."],
  ["Review Workbench", "复核工作台"],
  ["Use this as the single entry point for human review. Clinical necessity and QA feedback stay separate, but operators do not need two top-level menus.", "这里是人工复核的统一入口。医疗必要性和 QA 反馈保持分离，但运营人员不需要两个顶层菜单。"],
  ["Human review", "人工复核"],
  ["Correction Review", "修正复核"],
  ["Medical Review", "医疗复核"],
  ["Resolve clinical reasonableness, necessity, and documentation questions.", "处理临床合理性、必要性和资料问题。"],
  ["Open clinical queue", "打开医疗队列"],
  ["QA Review", "QA 复核"],
  ["Close sampled findings, reviewer disagreement, and feedback calibration.", "闭环抽样发现、复核分歧和反馈校准。"],
  ["Open QA queue", "打开 QA 队列"],
  ["Bootstrap Ops", "标签证据准备"],
  ["Training Label Handoff", "训练标签交付"],
  ["Prepare replay findings, missing-evidence requests, and reviewed labels as an audited handoff for the independent training platform.", "将历史回放发现、补件请求和已复核标签整理成可审计交付包，供独立训练平台使用。"],
  ["Label Evidence Handoff", "标签证据交付"],
  ["Label Evidence Source", "标签证据来源"],
  ["Using the configured pilot operations principal for historical replay, evidence requests, and label handoff governance.", "使用已配置的试点运营身份执行历史回放、补件请求和标签交付治理。"],
  ["Create backfill", "创建回放"],
  ["Generate evidence requests", "生成补件请求"],
  ["Evidence Intake", "证据接收"],
  ["Choose a specific request and link actual document evidence before changing its status.", "选择具体请求并关联真实文件证据后，再变更状态。"],
  ["Evidence request", "补件请求"],
  ["Evidence document refs", "证据文件引用"],
  ["Evidence notes", "证据备注"],
  ["Mark selected request received", "标记所选请求已接收"],
  ["Label Review", "标签复核"],
  ["Review selected label", "复核所选标签"],
  ["Review notes", "复核备注"],
  ["Load label handoff queues to inspect replay, evidence, and reviewed-label readiness.", "加载标签交付队列以查看回放、证据和标签复核准备状态。"],
  ["Loading label handoff queues...", "正在加载标签交付队列..."],
  ["Actions write audit events; suspicious leads and missing evidence stay out of the training handoff until reviewed.", "操作会写入审计事件；可疑线索和缺失证据在复核前不会进入训练交付包。"],
  ["Submitting label handoff action...", "正在提交标签交付操作..."],
  ["Historical Replay", "历史回放"],
  ["No backfill jobs yet.", "暂无回放任务。"],
  ["Evidence Requests", "补件请求"],
  ["No generated evidence requests yet.", "暂无已生成的补件请求。"],
  ["No reviewed-label handoff candidates yet.", "暂无待交付标签候选。"],
  ["Select request", "选择请求"],
  ["Select label item", "选择标签项"],
  ["Select the request before recording received evidence.", "记录收到证据前请先选择请求。"],
  ["Selected evidence request", "已选补件请求"],
  ["Selected evidence request is no longer in the queue.", "已选补件请求已不在队列中。"],
  ["Select the item before writing a governed label handoff review.", "写入受治理标签交付复核前请先选择标签项。"],
  ["Selected label item", "已选标签项"],
  ["Selected label item is no longer in the queue.", "已选标签项已不在队列中。"],
  ["Rule & Model Discovery Review", "规则与模型发现评审"],
  ["Use this as the single business entry for ML-discovered rule candidates and provider-trained model versions. Operators compare evidence, run backtests, inspect shadow gates, then accept or reject before anything can affect routing.", "这里是模型发现规则候选和 Provider 训练模型版本的统一业务入口。运营人员比较证据、运行回测、检查影子关卡后，才能接受或拒绝，任何内容都不能先影响路由。"],
  ["ML governance control", "模型治理控制"],
  ["Candidate Review Path", "候选评审路径"],
  ["Every candidate must show source, backtest or evaluation evidence, shadow comparison, review-capacity impact, human decision, and rollback path before it can affect routing.", "每个候选在影响路由前，都必须展示来源、回测或评估证据、影子对比、复核容量影响、人工决策和回滚路径。"],
  ["human approval required", "需要人工审批"],
  ["What Operators Decide Here", "运营在这里决策什么"],
  ["Business users do not tune raw features or train models here. They accept or reject explainable candidates based on backtest, shadow, and governance evidence.", "业务用户不在这里调原始特征或训练模型，只根据回测、影子运行和治理证据接受或拒绝可解释候选。"],
  ["release governance only", "仅发布治理"],
  ["ML rule intake", "模型规则接入"],
  ["Model intake", "模型接入"],
  ["Not here", "不在这里做"],
  ["ML Rule Review Queue", "模型规则评审队列"],
  ["Review discovered rules", "评审发现规则"],
  ["Provider Model Queue", "Provider 模型队列"],
  ["Review model evidence", "评审模型证据"],
  ["Routing Impact", "路由影响"],
  ["Check impact", "检查影响"],
  ["Evidence Package", "证据包"],
  ["Validate evidence", "验证证据"],
  ["Release History", "发布历史"],
  ["Open governance", "打开治理"],
  ["Evidence Hub", "证据中心"],
  ["Look up the evidence an investigator needs before making a case decision. This keeps context lookup separate from scoring and review actions.", "在做案件决策前，查询调查人员需要的证据。上下文查询与评分、复核动作保持分离。"],
  ["Context lookup", "上下文查询"],
  ["Document packet", "文件包"],
  ["redacted + traceable", "已脱敏 + 可追溯"],
  ["Evidence boundary", "证据边界"],
  ["LLM sees references, not raw claims text", "LLM 只看到引用，不看到原始理赔文本"],
  ["The runtime stores provenance, redaction state, retrieval purpose, and actor scope before Agent or QA views consume the packet.", "运行时会先保存来源、脱敏状态、检索目的和操作人范围，再供 Agent 或 QA 视图使用。"],
  ["Register", "登记"],
  ["document metadata", "文件元数据"],
  ["redacted output", "脱敏输出"],
  ["source spans", "来源片段"],
  ["Embed", "Embedding"],
  ["job state", "任务状态"],
  ["retrieval trail", "检索轨迹"],
  ["Evidence Runtime", "证据运行时"],
  ["Open runtime", "打开运行时"],
  ["Register document packets, chunks, OCR outputs, embedding jobs, and retrieval audit metadata.", "登记文件包、切块、OCR 输出、embedding 任务和检索审计元数据。"],
  ["Open provider graph signals, suspicious patterns, and network flags.", "查看 Provider 图谱信号、可疑模式和网络标记。"],
  ["Review provider", "复核 Provider"],
  ["Inspect member-level utilization, policy, and claim history context.", "查看会员层级的使用情况、保单和理赔历史上下文。"],
  ["Review member", "复核会员"],
  ["Search evidence", "搜索证据"],
  ["Search confirmed evidence without crossing adjudication boundaries.", "搜索已确认案例证据，但不越过理赔裁决边界。"],
  ["Check dataset lineage, schema mapping, and evaluation inputs.", "检查数据集血缘、Schema 映射和评估输入。"],
  ["Review data", "复核数据"],
  ["Operate the AI evidence metadata lifecycle without exposing raw document text or embedding vectors to the browser.", "在不向浏览器暴露原始文件文本或 embedding 向量的前提下，操作 AI 证据元数据生命周期。"],
  ["AI Evidence Foundation", "AI 证据底座"],
  ["Runtime Source", "运行时数据源"],
  ["Selected document id", "所选文件 ID"],
  ["leave blank to use first document", "留空则使用第一个文件"],
  ["Refresh evidence", "刷新证据"],
  ["Registering...", "登记中..."],
  ["Run demo evidence lifecycle", "运行演示证据生命周期"],
  ["Demo lifecycle writes document, chunk, OCR, embedding job, retrieval audit, and governance audit events.", "演示生命周期会写入文件、切块、OCR、embedding 任务、检索审计和治理审计事件。"],
  ["Registering governed evidence metadata...", "正在登记受治理的证据元数据..."],
  ["Load evidence runtime metadata to inspect the current governed packet state.", "加载证据运行时元数据以查看当前受治理文件包状态。"],
  ["Loading evidence runtime metadata...", "正在加载证据运行时元数据..."],
  ["Document Packets", "文件包"],
  ["No evidence documents registered for this customer scope.", "该客户范围暂无已登记证据文件。"],
  ["Selected Document Outputs", "所选文件输出"],
  ["Selected Document", "所选文件"],
  ["Chunks", "切块"],
  ["OCR Outputs", "OCR 输出"],
  ["No chunk metadata returned.", "未返回切块元数据。"],
  ["No OCR metadata returned.", "未返回 OCR 元数据。"],
  ["Embedding And Retrieval Audit", "Embedding 与检索审计"],
  ["Embedding Jobs", "Embedding 任务"],
  ["No embedding jobs registered.", "暂无已登记 embedding 任务。"],
  ["Retrieval Audit Events", "检索审计事件"],
  ["No retrieval audit events recorded.", "暂无检索审计事件。"],
  ["Boundary", "边界"],
  ["no raw text in UI", "UI 不展示原始文本"],
  ["Runtime Scoring", "实时评分"],
  ["Validate the claim scoring contract and inspect audit-backed routing output. Business reviewers should work from Dashboard, Leads & Cases, or Review Workbench.", "验证理赔评分契约并检查有审计支撑的路由输出。业务复核人员应从仪表盘、线索与案件或复核工作台进入。"],
  ["Integration Tool", "集成工具"],
  ["Scoring Request", "评分请求"],
  ["Dev API key", "开发 API Key"],
  ["Stored claim", "已存案件"],
  ["Full payload", "完整载荷"],
  ["Request JSON", "请求 JSON"],
  ["Validate scoring contract", "验证评分契约"],
  ["Validating...", "验证中..."],
  ["Scoring Response", "评分响应"],
  ["Submit a stored claim or full payload to validate response shape, route, audit trace, and evidence references.", "提交已存案件或完整载荷，验证响应结构、路由、审计轨迹和证据引用。"],
  ["Validating scoring contract...", "正在验证评分契约..."],
  ["Risk Score", "风险分"],
  ["Risk Level", "风险等级"],
  ["Review Required", "需要复核"],
  ["Review Mode", "复核模式"],
  ["Reason Code", "原因代码"],
  ["Run", "运行"],
  ["Risk Signal Breakdown", "风险信号拆解"],
  ["Alerts And Top Reasons", "告警与主要原因"],
  ["No top reasons returned.", "未返回主要原因。"],
  ["Model Output", "模型输出"],
  ["Evidence And Agent Prefill", "证据与 Agent 预填"],
  ["Similar Cases", "相似案件"],
  ["Routing and clinical payload", "路由与临床载荷"],
  ["Input Contract", "输入契约"],
  ["Stored claim ID or canonical claim payload", "已存案件 ID 或标准理赔载荷"],
  ["Request", "请求"],
  ["Contract", "契约"],
  ["required IDs, payload shape, tenant scope", "必填 ID、载荷结构、租户范围"],
  ["Signals", "信号"],
  ["Risk context", "风险上下文"],
  ["rules, model, provider, clinical evidence", "规则、模型、Provider、临床证据"],
  ["Policy", "策略"],
  ["Routing", "路由"],
  ["manual review, case creation, or watchlist", "人工复核、建案或观察名单"],
  ["Audit", "审计"],
  ["Trace", "追踪"],
  ["run_id, audit_id, evidence_refs", "run_id、audit_id、evidence_refs"],
  ["Queue", "队列"],
  ["Human work", "人工处理"],
  ["reviewers decide; system never denies alone", "复核人员决策；系统绝不单独拒赔"],
  ["This page validates runtime output; it is not the claim adjudication desk.", "本页用于验证运行时输出，不是理赔裁决工作台。"],
  ["Every response must carry route, reason, run_id, audit_id, and evidence_refs.", "每个响应都必须包含路由、原因、run_id、audit_id 和 evidence_refs。"],
  ["Routing outcome", "路由结果"],
  ["Signal Contract Map", "信号契约图"],
  ["Controls", "控制"],
  ["Model", "模型"],
  ["Clinical", "临床"],
  ["Provider graph", "Provider 图谱"],
  ["Knowledge", "知识"],
  ["Evidence", "证据"],
  ["Intake Ops", "进件处理"],
  ["Leads & Cases", "线索与案件"],
  ["Provider Risk", "Provider 风险"],
  ["Member Profile", "会员画像"],
  ["Knowledge Base", "知识库"],
  ["Data Sources", "数据源"],
  ["Rules", "规则"],
  ["Models", "模型"],
  ["Routing Policies", "路由策略"],
  ["Factor Factory", "因子工厂"],
  ["Audit Sampling", "审计抽样"],
  ["Governance", "治理"],
  ["Agent Investigator", "辅助调查"],
  ["Provider Model Intake", "Provider 模型接入"],
  ["Claim", "案件"],
  ["Claims", "案件"],
  ["Case", "案件"],
  ["Action", "动作"],
  ["Decision", "决策"],
  ["Authority", "权限依据"],
  ["Confidence", "置信度"],
  ["Evidence Refs", "证据引用"],
  ["Evidence refs", "证据引用"],
  ["Evidence requests", "补件请求"],
  ["Backfills", "回放任务"],
  ["Open labels", "开放标签"],
  ["Missing", "缺失"],
  ["Items", "项目"],
  ["Features", "特征"],
  ["Scope", "范围"],
  ["Retention", "留存"],
  ["Document", "文件"],
  ["Chunk", "切块"],
  ["Embedding", "Embedding"],
  ["Retrieval", "检索"],
  ["OCR", "OCR"],
  ["Risk", "风险"],
  ["risk", "风险"],
  ["Created", "已创建"],
  ["Completed", "已完成"],
  ["Active", "启用中"],
  ["Blocked", "已阻塞"],
  ["Blocking", "阻塞项"],
  ["Warnings", "警告"],
  ["Errors", "错误"],
  ["Data Quality", "数据质量"],
  ["Configured", "已配置"],
  ["Confirmed", "已确认"],
  ["Approved", "已批准"],
  ["Approve", "批准"],
  ["Activate", "启用"],
  ["Accepted", "已接受"],
  ["Dismissed", "已驳回"],
  ["Pending", "待处理"],
  ["Review", "复核"],
  ["Release", "发布"],
  ["Shadow", "影子运行"],
  ["Candidate", "候选"],
  ["Candidates", "候选"],
  ["Evaluation", "评估"],
  ["Evaluations", "评估"],
  ["Dataset", "数据集"],
  ["Datasets", "数据集"],
  ["Fields", "字段"],
  ["Findings", "发现"],
  ["Conclusion", "结论"],
  ["Actor", "操作人"],
  ["Endpoint", "端点"],
  ["Event", "事件"],
  ["Audit Events", "审计事件"],
  ["Audit Event Log", "审计事件日志"],
  ["Audit ID", "审计 ID"],
  ["Agent", "Agent"],
  ["Approvals", "审批"],
  ["Action accepted by API. Workspace refresh has been requested.", "API 已接受操作，工作台刷新已发起。"],
  ["API server is unavailable. Start the API server on 127.0.0.1:8080, then refresh this workspace.", "API 服务不可用。请先启动 127.0.0.1:8080 上的 API 服务，然后刷新当前工作台。"],
  ["No distribution records.", "暂无分布记录。"],
  ["Next actions", "下一步动作"],
  ["click to work", "点击处理"],
  ["FWA operating map", "FWA 运营图"],
  ["PRD runtime topology", "PRD 运行拓扑"],
  ["Risk operations matrix", "风险运营矩阵"],
  ["Rule command path", "规则指令路径"],
  ["FWA Rule Pack Matrix", "FWA 规则包矩阵"],
  ["Rule engine", "规则引擎"],
  ["rule pack", "规则包"],
  ["Human-safe lifecycle", "人工安全生命周期"],
  ["Model Monitoring Cockpit", "模型监控驾驶舱"],
  ["Active candidate", "启用候选"],
  ["Release gate", "发布关卡"],
  ["Promotion readiness", "晋级就绪度"],
  ["Gate pass", "关卡通过"],
  ["Label readiness", "标签就绪度"],
  ["Clinical evidence cockpit", "临床证据驾驶舱"],
  ["Clinical trace", "临床追踪"],
  ["Controlled outcomes", "受控结论"],
  ["Data Lineage Cockpit", "数据血缘驾驶舱"],
  ["Governed contract", "受治理契约"],
  ["Evaluation evidence", "评估证据"],
  ["Pilot blocker signal", "试点阻塞信号"],
  ["Queue Handoff", "队列交接"],
  ["Not released", "未发布"],
  ["Waiting for intake check", "等待进件检查"],
  ["Release in progress", "发布中"],
  ["Creating queue handoff", "正在创建队列交接"],
  ["Released", "已发布"],
  ["Claim entered downstream queue", "案件已进入下游队列"],
  ["Queue Route", "队列路由"],
  ["Live demo running", "现场演示运行中"],
  ["Live demo stopped", "现场演示已停止"],
  ["Live demo complete", "现场演示完成"],
  ["Normalizing, scoring, opening case, and writing back outcome", "正在标准化、评分、建案并回写结果"],
  ["Fix the runtime before presenting", "演示前请先修复运行时"],
  ["Inbox run", "收件运行"],
  ["Score run", "评分运行"],
  ["Lead", "线索"],
  ["Investigation audit", "调查审计"],
  ["Dashboard value", "仪表盘价值"]
]);

const translatedValues = new Set(translations.values());
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
    #[wasm_bindgen(js_name = applyDocumentLanguage)]
    pub(crate) fn apply_document_language(language: &str);
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
        "Routing Policies" => tr(language, "Routing Policies", "路由策略"),
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
            "Inspect routing boundaries for model and policy execution.",
            "检查模型与政策执行的路由边界。",
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
        "Routing Policies" => tr(language, "execution routing", "执行路由"),
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
