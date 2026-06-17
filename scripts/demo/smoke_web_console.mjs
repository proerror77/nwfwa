#!/usr/bin/env node
import { access, readdir, readFile } from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "../..");
const webDir = path.join(repoRoot, "apps/web-console");
const srcDir = path.join(webDir, "src");
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

async function collectSourceText(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const chunks = [];
  for (const entry of entries) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      chunks.push(await collectSourceText(entryPath));
    } else if (entry.name.endsWith(".rs")) {
      chunks.push(await readFile(entryPath, "utf8"));
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
    const sourceText = await collectSourceText(srcDir);
    const bundleText = await collectBuiltText(distDir);
    const builtText = `${bundleText}\n${sourceText}`;

    for (const expected of [
      "FWA PLATFORM",
      "FWA Operations",
      "Claims FWA Operations Console",
      "FWA Platform",
      "Live operations",
      "实时运营",
      "主工作流",
      "页面路径",
      "hashchange",
    ]) {
      assertContains(builtText, expected, "web console shell bundle");
    }

    for (const expectedPage of [
      "Operations Dashboard",
      "Claims Triage Queue",
      "Investigation Workbench",
      "Case Tracker",
      "Evidence Center",
      "Evidence Runtime",
      "Member Profile",
      "Provider Risk",
      "Knowledge Base",
      "Data Sources",
      "AI Investigator",
      "Rule Library",
      "Model Governance",
      "Review Routing Policies",
      "Quality & Governance",
      "Audit Sampling",
      "Medical Review",
      "QA Feedback",
    ]) {
      assertContains(builtText, expectedPage, "web console active route bundle");
    }

    for (const expectedSlug of [
      "dashboard",
      "claims",
      "review",
      "cases",
      "evidence",
      "evidence-runtime",
      "member",
      "provider",
      "knowledge",
      "data-sources",
      "agent",
      "rules",
      "models",
      "routing",
      "governance",
      "audit",
      "medical",
      "qa",
    ]) {
      assertContains(sourceText, `"${expectedSlug}"`, "web console active route slug");
    }

    for (const expectedPanel of [
      "Claims today",
      "SLA compliance",
      "Claims Triage Queue",
      "Investigation Queue",
      "Risk Signals",
      "Provider 风险分析",
      "AI Investigation Summary",
      "Document Packets",
      "Embedding And Retrieval Audit",
      "Confirmed Knowledge Cases",
      "Similar Case Search",
      "Knowledge graph match",
      "Confirmed Evidence",
      "Data Foundation Control",
      "Data Lineage Cockpit",
      "Field Mapping Lineage",
      "Rule Library",
      "Rule Discovery Workbench",
      "Tree Depth",
      "Backtest Evidence",
      "Rule Promotion Gates",
      "Candidate rule workflow",
      "shadow evidence ready",
      "FWA Rule Pack Matrix",
      "Model Monitoring Cockpit",
      "Model telemetry map",
      "Review Routing Policies",
      "QA Queue Summary",
      "QA Sampling Governance",
      "Medical Review Queue",
      "Clinical Signals",
      "Human Clinical Decision",
      "Sampling Governance Map",
      "Investigation Package",
      "Evidence Status",
      "Assistive Boundary",
      "Agent investigation blueprint",
    ]) {
      assertContains(builtText, expectedPanel, "web console active panel bundle");
    }

    for (const removedLegacySurface of [
      "ClaimInboxPage",
      "RuntimeScoringPage",
      "BootstrapOpsPage",
      "MlopsWorkspacePage",
      "FactorFactoryPage",
      "GovernanceSnapshot",
      "Training Label Handoff",
      "Live TPA Demo Run",
      "Correction Worklist",
      "Runtime Scoring",
      "Provider Model Intake",
      "Leads & Cases",
      "Discovery Review",
    ]) {
      assertNotContains(sourceText, removedLegacySurface, "web console pruned legacy source");
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
