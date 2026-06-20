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
      // New 4-page navigation labels
      "Overview",
      "Action Queue",
      "Investigate",
      "System Governance",
      "运营概况",
      "需要处理",
      "调查工作台",
      "系统治理",
      "hashchange",
    ]) {
      assertContains(builtText, expected, "web console shell bundle");
    }

    for (const expectedPage of [
      // New 4-page navigation
      "Overview",
      "Action Queue",
      "Investigate",
      "System Governance",
      // Content still present in the reused inner pages
      "Rule Library",
      "Model Governance",
      "QA Feedback",
    ]) {
      assertContains(builtText, expectedPage, "web console active route bundle");
    }

    for (const expectedSlug of [
      // New 4 slugs
      "dashboard",
      "queue",
      "investigate",
      "governance",
    ]) {
      assertContains(sourceText, `"${expectedSlug}"`, "web console active route slug");
    }

    for (const expectedPanel of [
      // Dashboard panels (new)
      "Prevention today",
      "Prevented today",
      "Action needed",
      "Live intake",
      "Precision",
      "SLA compliance",
      // Inner pages still in bundle (unchanged)
      "Risk Signals",
      "Provider 风险分析",
      "AI Investigation Summary",
      "Document Packets",
      "Confirmed Knowledge Cases",
      "Similar Case Search",
      "Data Foundation Control",
      "Rule Library",
      "Rule Discovery Workbench",
      "Backtest Evidence",
      "Rule Promotion Gates",
      "Model Monitoring Cockpit",
      "Review Routing Policies",
      "QA Queue Summary",
      "Medical Review Queue",
      "Investigation Package",
      "Evidence Status",
      "Assistive Boundary",
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
