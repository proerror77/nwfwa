#!/usr/bin/env node
import { spawn } from "node:child_process";
import { access, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "../..");
const webDir = path.join(repoRoot, "apps/web-console");
const distDir = path.join(webDir, "dist");
const viteBin = path.join(webDir, "node_modules/vite/bin/vite.js");
const port = Number(process.env.WEB_CONSOLE_SMOKE_PORT ?? 4173);
const baseUrl = `http://127.0.0.1:${port}`;

async function requireBuiltArtifact() {
  await access(path.join(distDir, "index.html"));
  await access(viteBin);
}

async function fetchText(url) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`${url} returned HTTP ${response.status}`);
  }
  return response.text();
}

async function waitForPreview() {
  let lastError;
  for (let attempt = 0; attempt < 60; attempt += 1) {
    try {
      return await fetchText(baseUrl);
    } catch (error) {
      lastError = error;
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  }
  throw lastError ?? new Error("web console preview did not start");
}

function assertContains(value, expected, label) {
  if (!value.includes(expected)) {
    throw new Error(`expected ${label} to contain ${expected}`);
  }
}

async function main() {
  await requireBuiltArtifact();

  const preview = spawn(
    process.execPath,
    [viteBin, "preview", "--host", "127.0.0.1", "--port", String(port), "--strictPort"],
    { cwd: webDir, stdio: ["ignore", "pipe", "pipe"] },
  );
  let previewOutput = "";
  preview.stdout.on("data", (chunk) => {
    previewOutput += chunk.toString();
  });
  preview.stderr.on("data", (chunk) => {
    previewOutput += chunk.toString();
  });

  try {
    const html = await waitForPreview();
    assertContains(html, '<div id="root">', "index HTML");
    const moduleMatch = html.match(/<script[^>]+type="module"[^>]+src="([^"]+\.js)"/);
    if (!moduleMatch) {
      throw new Error("index HTML does not reference a module JS asset");
    }

    const assetUrl = new URL(moduleMatch[1], baseUrl).toString();
    const bundle = await fetchText(assetUrl);
    assertContains(bundle, "FWA Studio", "web console bundle");
    assertContains(bundle, "Runtime Scoring", "web console bundle");

    const builtHtml = await readFile(path.join(distDir, "index.html"), "utf8");
    assertContains(builtHtml, moduleMatch[1], "built index HTML");
  } finally {
    preview.kill();
  }

  if (preview.exitCode && preview.exitCode !== 0) {
    throw new Error(`vite preview exited unexpectedly:\n${previewOutput}`);
  }
}

main().catch((error) => {
  console.error(`web console smoke failed: ${error.message}`);
  process.exit(1);
});
