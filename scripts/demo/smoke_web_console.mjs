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
    const builtText = await collectBuiltText(distDir);
    assertContains(builtText, "FWA Studio", "web console bundle");
    assertContains(builtText, "Claim Inbox", "web console bundle");
    assertContains(builtText, "Correction Review", "web console bundle");
    assertContains(builtText, "Runtime Scoring", "web console bundle");
    assertContains(builtText, "Model Performance", "web console bundle");
    assertContains(builtText, "Promotion Gates", "web console bundle");
    assertContains(builtText, "Retraining Readiness", "web console bundle");
    for (const expectedModule of [
      "Dashboard",
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
    ]) {
      assertContains(builtText, expectedModule, "web console navigation bundle");
    }
    for (const expectedPanel of [
      "Management Dashboard",
      "Executive KPIs",
      "Value Measurement",
      "ROI Attribution",
      "Seven-Layer Coverage",
      "Claim Scoring API",
      "Scoring Request",
      "Scoring Decision",
      "Seven-Layer Runtime Scores",
      "Alerts And Top Reasons",
      "Model Output",
      "Evidence And Agent Prefill",
      "Rule Library",
      "Rule Performance",
      "Rule Promotion Readiness",
      "Backtest Evidence",
      "Rule Promotion Gates",
      "Discovery Mode",
      "Candidate Source",
      "Threshold Integrity",
      "Model Governance",
      "Deployment Boundary",
      "Profile Evidence",
      "Candidate Governance",
      "promotion_review_ready",
      "Risk Fusion Routing",
      "Routing Policy Control",
      "Routing Policy Inventory",
      "Routing Promotion Gates",
      "Data Source Control",
      "Dataset Catalog",
      "Dataset Health",
      "Split And Schema Coverage",
      "Field Mapping Lineage",
      "Model Evaluation Lineage",
      "Factor Cards",
      "AUC Gain",
      "Field Governance",
      "Leakage Candidates",
      "Case Workflow",
      "Lead Triage",
      "Case Status Update",
      "Profile Summary API",
      "Member Profile Source",
      "Member Profile Summary",
      "Profile Narrative",
      "SLA Breached",
      "QA Queue",
      "QA Queue Summary",
      "Feedback Closure",
      "Review Findings",
      "Canonical Evidence",
      "Calibration Signal",
      "Promotion Gate Governance",
      "API Call Records",
      "Audit Event Log",
      "Agent Run Logs",
      "Assistive Boundary",
      "Guardrail Boundary",
      "Human Gate",
      "Graph Risk",
      "Clinical Signals",
      "Medical Review Queue",
      "Clinical Outcomes",
      "Evidence Status",
      "Layer Coverage",
      "Knowledge Base",
      "Confirmed Knowledge Cases",
      "Similar Case Search",
      "Evidence Provenance",
      "Graph Evidence Status",
      "Confirmed Evidence",
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
