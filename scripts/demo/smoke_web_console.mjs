#!/usr/bin/env node
import { access, readdir, readFile } from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "../..");
const webDir = path.join(repoRoot, "apps/web-console");
const distDir = path.join(webDir, "dist");
const port = Number(process.env.WEB_CONSOLE_SMOKE_PORT ?? 4173);
const baseUrl = `http://127.0.0.1:${port}`;

async function requireBuiltArtifact() {
  await access(path.join(distDir, "index.html"));
}

async function collectBuiltText(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const chunks = [];
  for (const entry of entries) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      chunks.push(await collectBuiltText(entryPath));
    } else if (/\.(html|js|wasm|css)$/.test(entry.name)) {
      chunks.push((await readFile(entryPath)).toString("utf8"));
    }
  }
  return chunks.join("\n");
}

async function fetchText(url) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`${url} returned HTTP ${response.status}`);
  }
  return response.text();
}

async function waitForServer() {
  let lastError;
  for (let attempt = 0; attempt < 60; attempt += 1) {
    try {
      return await fetchText(baseUrl);
    } catch (error) {
      lastError = error;
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  }
  throw lastError ?? new Error("web console static server did not start");
}

function assertContains(value, expected, label) {
  if (!value.includes(expected)) {
    throw new Error(`expected ${label} to contain ${expected}`);
  }
}

function assertNotContains(value, unexpected, label) {
  if (value.includes(unexpected)) {
    throw new Error(`expected ${label} not to contain ${unexpected}`);
  }
}

function assertMatches(value, expected, label) {
  if (!expected.test(value)) {
    throw new Error(`expected ${label} to match ${expected}`);
  }
}

async function main() {
  await requireBuiltArtifact();

  const server = http.createServer(async (request, response) => {
    const url = new URL(request.url ?? "/", baseUrl);
    const relativePath = url.pathname === "/" ? "index.html" : url.pathname.slice(1);
    const filePath = path.normalize(path.join(distDir, relativePath));
    if (!filePath.startsWith(distDir)) {
      response.writeHead(403).end();
      return;
    }
    try {
      response.end(await readFile(filePath));
    } catch {
      response.writeHead(404).end();
    }
  });
  await new Promise((resolve) => server.listen(port, "127.0.0.1", resolve));

  try {
    const html = await waitForServer();
    assertMatches(html, /<div\s+id="?root"?><\/div>/, "index HTML");
    const sourceText = await readFile(path.join(webDir, "src/main.rs"), "utf8");
    const builtText = await collectBuiltText(distDir);
    assertContains(builtText, "FWA Studio", "web console bundle");
    assertContains(builtText, "NOVA FWA", "web console bundle");
    assertContains(builtText, "FWA Platform", "web console bundle");
    assertContains(builtText, "Operations desk", "web console visual shell bundle");
    assertContains(builtText, "Daily Work", "web console navigation bundle");
    assertContains(builtText, "Control Rooms", "web console navigation bundle");
    assertContains(builtText, "Intake Ops", "web console navigation bundle");
    assertContains(builtText, "中文", "web console language toggle bundle");
    assertContains(builtText, "实时运营", "web console language toggle bundle");
    assertContains(builtText, "日常作业", "web console bilingual navigation bundle");
    assertContains(builtText, "控制室", "web console bilingual navigation bundle");
    assertContains(builtText, "线索与案件", "web console bilingual navigation bundle");
    assertNotContains(builtText, "TPA Demo Runner", "web console business navigation bundle");
    assertContains(builtText, "Discovery Review", "web console navigation bundle");
    assertNotContains(builtText, "Advanced", "web console business navigation bundle");
    assertContains(builtText, "Provider Model Intake", "web console model release drilldown bundle");
    assertContains(builtText, "Rule & Model Discovery Review", "web console detection review bundle");
    assertContains(builtText, "ML Rule Review Queue", "web console detection review bundle");
    assertContains(builtText, "Provider Model Queue", "web console detection release bundle");
    assertContains(builtText, "No ad hoc model training", "web console detection release boundary bundle");
    assertContains(builtText, "hashchange", "web console browser history bundle");
    assertContains(builtText, "leads-cases", "web console browser history bundle");
    assertContains(builtText, "discovery-review", "web console browser history bundle");
    assertContains(builtText, "Bootstrap Ops", "web console navigation bundle");
    assertContains(builtText, "Historical Replay", "web console bootstrap ops bundle");
    assertContains(builtText, "Backfill evidence detail", "web console bootstrap ops bundle");
    assertContains(builtText, "Evidence Requests", "web console bootstrap ops bundle");
    assertContains(builtText, "Bootstrap evidence detail", "web console bootstrap ops bundle");
    assertContains(builtText, "Bootstrap label detail", "web console bootstrap ops bundle");
    assertContains(builtText, "Label Bootstrap", "web console bootstrap ops bundle");
    assertContains(builtText, "Create backfill", "web console bootstrap ops bundle");
    assertContains(builtText, "Generate evidence requests", "web console bootstrap ops bundle");
    assertContains(builtText, "Evidence Intake", "web console bootstrap ops bundle");
    assertContains(builtText, "Label Review", "web console bootstrap ops bundle");
    assertContains(builtText, "Mark selected request received", "web console bootstrap ops bundle");
    assertContains(builtText, "Review selected label", "web console bootstrap ops bundle");
    assertContains(builtText, "Provider candidate release", "web console provider model intake bundle");
    assertContains(builtText, "Provider Candidate Release Control", "web console provider model intake bundle");
    assertContains(builtText, "Offline Training Handoff", "web console mlops workspace bundle");
    assertContains(builtText, "Monitoring Review Queue", "web console mlops workspace bundle");
    assertContains(builtText, "Submitted monitoring reports open human review tasks", "web console mlops review queue bundle");
    assertContains(builtText, "No monitoring review tasks returned for this model.", "web console mlops review queue bundle");
    assertContains(builtText, "Monitoring decision", "web console mlops review action bundle");
    assertContains(builtText, "Submit monitoring decision", "web console mlops review action bundle");
    assertContains(builtText, "Reject task", "web console mlops review action bundle");
    assertContains(builtText, "Prepare retraining from task", "web console mlops review action bundle");
    assertContains(builtText, "Open rollback review", "web console mlops review action bundle");
    assertContains(builtText, "Alert Delivery Queue", "web console mlops alert delivery bundle");
    assertContains(builtText, "Submit alert decision", "web console mlops alert delivery bundle");
    assertContains(builtText, "Escalate alert to review", "web console mlops alert delivery bundle");
    assertContains(builtText, "Provider Training Job Operations", "web console mlops training job bundle");
    assertContains(builtText, "Claim next queued job", "web console mlops training job bundle");
    assertContains(builtText, "Update job status", "web console mlops training job bundle");
    assertContains(builtText, "Register provider output", "web console mlops training job bundle");
    assertContains(builtText, "Candidate artifact SHA", "web console mlops provider output bundle");
    assertContains(builtText, "Training artifact", "web console mlops provider output bundle");
    assertContains(builtText, "Serving manifest", "web console mlops provider output bundle");
    assertContains(builtText, "Candidate endpoint", "web console mlops provider output bundle");
    assertContains(builtText, "Candidate AUC", "web console mlops provider output bundle");
    assertContains(builtText, "Confusion matrix JSON", "web console mlops provider output bundle");
    assertContains(builtText, "Metrics JSON", "web console mlops provider output bundle");
    assertContains(builtText, "Mined rule candidates JSON", "web console mlops provider output bundle");
    assertContains(builtText, "Governed Actions", "web console mlops workspace bundle");
    assertContains(builtText, "Request provider retraining", "web console provider model action bundle");
    assertContains(builtText, "Submit release review", "web console provider model action bundle");
    assertContains(builtText, "Activate approved candidate", "web console mlops action bundle");
    assertContains(builtText, "Rollback active model", "web console mlops action bundle");
    assertContains(builtText, "Model Candidates", "web console mlops workspace bundle");
    assertContains(builtText, "Training Jobs", "web console mlops workspace bundle");
    assertContains(builtText, "Review Workbench", "web console navigation bundle");
    assertContains(builtText, "ML Rule Review Queue", "web console detection release bundle");
    assertContains(builtText, "Provider Model Queue", "web console detection release bundle");
    assertContains(builtText, "Routing Impact", "web console detection release bundle");
    assertContains(builtText, "Release History", "web console detection release bundle");
    assertContains(builtText, "Evidence Hub", "web console navigation bundle");
    assertContains(builtText, "Real-time operations", "web console workspace bundle");
    assertContains(builtText, "Scored leads", "web console global system map bundle");
    assertContains(builtText, "Human gate", "web console global system map bundle");
    assertContains(builtText, "Audit trail", "web console global system map bundle");
    assertContains(builtText, "Value proof", "web console global system map bundle");
    assertContains(builtText, "Risk Signal Breakdown", "web console risk signal visual bundle");
    assertContains(builtText, "downstream human queue", "web console risk signal visual bundle");
    assertContains(builtText, "Risk distribution", "web console dashboard visual bundle");
    assertContains(builtText, "Risk mix", "web console dashboard visual bundle");
    assertContains(builtText, "Pilot Operations", "web console dashboard visual bundle");
    assertContains(builtText, "confirmed prevented payment", "web console dashboard value wording bundle");
    assertContains(builtText, "Savings / review cost", "web console dashboard value wording bundle");
    assertContains(builtText, "Costs are tracked as pilot investment", "web console dashboard value wording bundle");
    assertNotContains(builtText, "Net value", "web console dashboard value wording bundle");
    assertNotContains(builtText, "ROI proof", "web console dashboard value wording bundle");
    assertNotContains(builtText, "ROI pending", "web console dashboard value wording bundle");
    assertNotContains(builtText, "ROI readiness", "web console dashboard value wording bundle");
    assertContains(builtText, "Next actions", "web console dashboard visual bundle");
    assertContains(builtText, "click to work", "web console dashboard visual bundle");
    assertContains(builtText, "FWA operating map", "web console dashboard topology bundle");
    assertContains(builtText, "PRD runtime topology", "web console dashboard topology bundle");
    assertContains(builtText, "Risk scoring service", "web console dashboard topology bundle");
    assertContains(builtText, "rules + risk mix", "web console dashboard topology bundle");
    assertContains(builtText, "human queue", "web console dashboard topology bundle");
    assertNotContains(builtText, "L2 rule hit", "web console dashboard topology bundle");
    assertContains(builtText, "Input Contract", "web console runtime illustration bundle");
    assertContains(builtText, "Signal Contract Map", "web console runtime signal illustration bundle");
    assertContains(builtText, "This page validates runtime output", "web console human gate illustration bundle");
    assertContains(builtText, "Triage", "web console dashboard queue bundle");
    assertContains(builtText, "Investigate", "web console dashboard queue bundle");
    assertContains(builtText, "Review", "web console dashboard queue bundle");
    assertContains(builtText, "Govern", "web console dashboard queue bundle");
    assertContains(builtText, "Open clinical queue", "web console review workbench bundle");
    assertContains(builtText, "Review discovered rules", "web console detection release bundle");
    assertContains(builtText, "Search evidence", "web console evidence workbench bundle");
    assertContains(builtText, "Rule command path", "web console rules visual bundle");
    assertContains(builtText, "Rule Discovery Workbench", "web console rule discovery workbench bundle");
    assertContains(builtText, "Mining Dataset URI", "web console rule discovery workbench bundle");
    assertContains(builtText, "Tree Depth", "web console rule discovery workbench bundle");
    assertContains(builtText, "Inline Labeled Evaluation Dataset", "web console rule discovery workbench bundle");
    assertContains(builtText, "Candidate rule workflow", "web console rule discovery workbench bundle");
    assertContains(builtText, "Submit shadow evidence", "web console rule shadow evidence bundle");
    assertContains(builtText, "Shadow evidence submitted for promotion gates.", "web console rule shadow evidence bundle");
    assertContains(builtText, "Accept for governance review", "web console rule discovery workbench bundle");
    assertContains(builtText, "Reject selected candidate", "web console rule discovery workbench bundle");
    assertContains(builtText, "Run backtest, then accept a candidate for governance review or reject it without saving a draft.", "web console rule candidate review gate bundle");
    assertContains(builtText, "needs human review", "web console rule discovery workbench bundle");
    assertContains(builtText, "draft candidate for governance review", "web console rule discovery workbench bundle");
    assertContains(sourceText, "saved_draft_rule_id", "web console rule discovery response handling");
    assertNotContains(builtText, "Draft Owner", "web console rule discovery owner boundary bundle");
    assertNotContains(
      sourceText,
      '<option value="completed">{"completed"}</option>',
      "web console manual training status options"
    );
    assertContains(builtText, "Discover candidates", "web console rule discovery workbench bundle");
    assertContains(builtText, "FWA Rule Pack Matrix", "web console rule pack visual bundle");
    assertContains(builtText, "duplicate billing", "web console rule pack visual bundle");
    assertContains(builtText, "medical necessity evidence gap", "web console rule pack visual bundle");
    assertContains(builtText, "Model Monitoring Cockpit", "web console model monitoring visual bundle");
    assertContains(builtText, "Shadow evidence", "web console model monitoring visual bundle");
    assertContains(builtText, "Label readiness", "web console model monitoring visual bundle");
    assertContains(builtText, "Agent investigation blueprint", "web console agent blueprint bundle");
    assertContains(builtText, "Governance locks", "web console agent blueprint bundle");
    assertContains(builtText, "no auto denial", "web console agent blueprint bundle");
    assertContains(builtText, "Model telemetry map", "web console models visual bundle");
    assertContains(builtText, "Queue Source", "web console leads cases queue bundle");
    assertContains(builtText, "Investigation Control", "web console leads cases queue bundle");
    assertContains(builtText, "SLA attention", "web console leads cases queue bundle");
    assertContains(builtText, "Waiting evidence", "web console business status label bundle");
    assertContains(builtText, "Pre-payment review", "web console business status label bundle");
    assertContains(builtText, "High risk", "web console business risk label bundle");
    assertContains(builtText, "Lead movement", "web console leads cases queue bundle");
    assertContains(builtText, "Case movement", "web console leads cases queue bundle");
    assertContains(builtText, "Generated Leads", "web console leads cases queue bundle");
    assertContains(builtText, "Investigation Cases", "web console leads cases queue bundle");
    assertContains(builtText, "Case Investigation Workspace", "web console leads cases action bundle");
    assertContains(builtText, "Selected lead", "web console leads cases action bundle");
    assertContains(builtText, "Selected case", "web console leads cases action bundle");
    assertContains(builtText, "Agent-assisted case package", "web console leads cases agent bundle");
    assertContains(builtText, "Generate case package", "web console leads cases agent bundle");
    assertContains(builtText, "Human writeback required", "web console leads cases human gate bundle");
    assertContains(builtText, "Case status maintenance", "web console leads cases maintenance bundle");
    assertContains(builtText, "Inbound Claim Packet", "web console claim inbox release bundle");
    assertContains(builtText, "Live TPA Demo Run", "web console live TPA demo bundle");
    assertContains(builtText, "Run full TPA demo", "web console live TPA demo bundle");
    assertContains(builtText, "TPA packet -> risk queue -> case -> value proof", "web console live TPA demo bundle");
    assertContains(builtText, "Correction Worklist", "web console claim inbox release bundle");
    assertContains(builtText, "Release accepted claim", "web console claim inbox release bundle");
    assertContains(builtText, "Queue Handoff", "web console claim inbox release bundle");
    assertContains(builtText, "Coverage limit needs correction", "web console claim inbox release bundle");
    assertContains(builtText, "Waiting for intake check", "web console claim inbox release bundle");
    assertContains(builtText, "Queue Ready", "web console claim inbox release bundle");
    assertContains(builtText, "Needs review", "web console claim inbox release bundle");
    assertContains(builtText, "Manual review", "web console business action label bundle");
    assertContains(builtText, "No automatic denial", "web console agent boundary label bundle");
    assertContains(builtText, "Medical review required", "web console clinical label bundle");
    assertContains(builtText, "Technical payload editor", "web console claim inbox release bundle");
    assertNotContains(builtText, "Approve for scoring", "web console claim inbox release bundle");
    assertNotContains(builtText, "Scoring Release", "web console claim inbox release bundle");
    assertContains(builtText, "Runtime Scoring", "web console bundle");
    assertContains(builtText, "Model Performance", "web console bundle");
    assertContains(builtText, "Promotion Gates", "web console bundle");
    assertContains(builtText, "Retraining Readiness", "web console bundle");
    for (const expectedModule of [
      "Dashboard",
      "Intake Ops",
      "Discovery Review",
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
      "Evidence Runtime",
      "Agent Investigator",
      "QA Review",
      "Governance",
    ]) {
      assertContains(builtText, expectedModule, "web console navigation bundle");
    }
    for (const expectedPanel of [
      "Management Dashboard",
      "Executive KPIs",
      "Value proof",
      "Confirmed prevented payment",
      "Investigation Writeback",
      "Confirm and write back",
      "Integration Tool",
      "Scoring Request",
      "Scoring Response",
      "Risk Signal Breakdown",
      "Alerts And Top Reasons",
      "Model Output",
      "Evidence And Agent Prefill",
      "Evidence Runtime",
      "AI Evidence Foundation",
      "Run demo evidence lifecycle",
      "Document Packets",
      "Selected Document Outputs",
      "Embedding And Retrieval Audit",
      "no raw text in UI",
      "Rule Library",
      "Rule library detail",
      "Additional rule library detail",
      "Rule Performance",
      "Performance detail",
      "Additional rule performance detail",
      "Rule Promotion Readiness",
      "Backtest Evidence",
      "Backtest evidence detail",
      "Rule Promotion Gates",
      "Rule promotion gate detail",
      "Discovery Mode",
      "Candidate Source",
      "Threshold Integrity",
      "Model Governance",
      "Provider Model Intake",
      "Governed Actions",
      "Offline Training Handoff",
      "Model Candidates",
      "Training Jobs",
      "Deployment Boundary",
      "Profile Evidence",
      "Candidate Governance",
      "promotion_review_ready",
      "Risk Fusion Routing",
      "Routing Policy Control",
      "Routing Policy Inventory",
      "Routing Promotion Gates",
      "Data Source Control",
      "Data Foundation Control",
      "registered sources",
      "Dataset Catalog",
      "Dataset Health",
      "Split And Schema Coverage",
      "Field Mapping Lineage",
      "Model Evaluation Lineage",
      "Evaluation evidence detail",
      "Factor Cards",
      "AUC Gain",
      "Field Governance",
      "Leakage Candidates",
      "Case Workflow",
      "Queue Source",
      "Lead Triage",
      "Case status maintenance",
      "Profile Summary API",
      "Member Profile Source",
      "Member Profile Summary",
      "Member Evidence Map",
      "Utilization Snapshot",
      "Policy exposure",
      "Profile Narrative",
      "Provider Graph Risk",
      "Provider Risk Source",
      "Provider Risk Summary",
      "Provider Risk Profiles",
      "Graph Risk Focus",
      "Provider Network",
      "Review route",
      "SLA Breached",
      "QA Queue",
      "QA Queue Summary",
      "Feedback Closure",
      "Review Findings",
      "Evidence package",
      "QA evidence detail",
      "Closure evidence detail",
      "QA feedback loop cockpit",
      "QA closed-loop routing",
      "Feedback closure path",
      "Canonical Evidence",
      "Calibration Signal",
      "Promotion Gate Governance",
      "Routing Decision Map",
      "Risk fusion and routing",
      "Confidence gate",
      "Human-safe route",
      "API Call Records",
      "Audit Event Log",
      "Agent Run Logs",
      "risk signal findings",
      "Governance control tower",
      "Audit-by-design map",
      "Payload trace detail",
      "API evidence detail",
      "Agent run detail",
      "Configuration check detail",
      "All blocking check detail",
      "Evidence Trace Hub",
      "Pilot Security Readiness",
      "Pilot Gate",
      "Blocking Checks",
      "Assistive Boundary",
      "Guardrail Boundary",
      "Human Gate",
      "Graph Risk",
      "Clinical Signals",
      "Clinical evidence cockpit",
      "Medical necessity path",
      "Controlled outcomes",
      "Human Clinical Decision",
      "Controlled outcome",
      "Confirm clinical review",
      "Medical Review Queue",
      "Clinical Outcomes",
      "Evidence Status",
      "Layer Coverage",
      "QA Sampling Governance",
      "Sampling Governance Map",
      "Deterministic seed",
      "Selected leads",
      "Sample output detail",
      "Selected lead detail",
      "Sample audit payload detail",
      "Audit trace",
      "Audit Sample Control",
      "Audit Sample Inventory",
      "Audit Sample Event Trace",
      "Knowledge Base",
      "Confirmed Knowledge Cases",
      "Similar Case Search",
      "Evidence Provenance",
      "Graph Evidence Status",
      "Knowledge graph match",
      "Structured + semantic retrieval",
      "Evidence provenance path",
      "Confirmed Evidence",
      "Assistive Investigation",
      "Investigation Request",
      "Investigation Package",
      "Investigation evidence detail",
      "Agent evidence orchestration",
      "Guardrail path",
      "assistive package only",
      "Agent Run Evidence Trail",
      "Agent Run Governance Map",
      "Agent run evidence detail",
      "Policy check",
      "Tool allowlist",
      "Human approval gate",
      "Evidence audit trail",
      "Source Trace",
      "Lineage",
      "Audit Coverage",
      "Canonical Trace Coverage",
      "Canonical Trace",
      "Canonical Trace Only",
      "Input Mode",
    ]) {
      assertContains(builtText, expectedPanel, "web console operations panel bundle");
    }

    const builtHtml = await readFile(path.join(distDir, "index.html"), "utf8");
    assertContains(builtHtml, "wasm", "built index HTML");
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
}

main().catch((error) => {
  console.error(`web console smoke failed: ${error.message}`);
  process.exit(1);
});
