import { spawn } from "node:child_process";
import { mkdir, readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";

const root = resolve(dirname(new URL(import.meta.url).pathname), "..");
const chromePath =
  process.env.CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const port = Number(process.env.PORT || 4177);
const baseUrl = `http://127.0.0.1:${port}`;
const artifactDir = resolve(root, "artifacts");

function wait(ms) {
  return new Promise((resolveWait) => setTimeout(resolveWait, ms));
}

function run(command, args, options = {}) {
  return new Promise((resolveRun, rejectRun) => {
    const child = spawn(command, args, {
      cwd: root,
      stdio: ["ignore", "pipe", "pipe"],
      ...options,
    });
    let stdout = "";
    let stderr = "";
    child.stdout?.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr?.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("error", rejectRun);
    child.on("exit", (code) => {
      if (code === 0) {
        resolveRun({ stdout, stderr });
      } else {
        rejectRun(new Error(`${command} ${args.join(" ")} failed with ${code}\n${stdout}\n${stderr}`));
      }
    });
  });
}

async function waitForServer() {
  for (let attempt = 0; attempt < 60; attempt += 1) {
    try {
      const response = await fetch(baseUrl);
      if (response.ok) return;
    } catch {
      // Vite preview is still starting.
    }
    await wait(500);
  }
  throw new Error(`Timed out waiting for ${baseUrl}`);
}

async function capture(viewport, outputName) {
  const screenshotPath = resolve(artifactDir, outputName);
  const { stdout } = await run(chromePath, [
    "--headless=new",
    "--use-gl=swiftshader",
    "--enable-unsafe-swiftshader",
    "--no-first-run",
    "--disable-default-apps",
    `--window-size=${viewport}`,
    "--virtual-time-budget=5000",
    `--screenshot=${screenshotPath}`,
    baseUrl,
  ]);
  return { screenshotPath, stdout };
}

async function dumpDom() {
  const { stdout } = await run(chromePath, [
    "--headless=new",
    "--use-gl=swiftshader",
    "--enable-unsafe-swiftshader",
    "--no-first-run",
    "--disable-default-apps",
    "--virtual-time-budget=5000",
    "--dump-dom",
    baseUrl,
  ]);
  return stdout;
}

await mkdir(artifactDir, { recursive: true });
await run("npm", ["run", "build"]);

const server = spawn("npm", ["run", "preview", "--", "--port", String(port)], {
  cwd: root,
  stdio: ["ignore", "pipe", "pipe"],
});

try {
  await waitForServer();
  await capture("1440,960", "opengeometry-demo-desktop.png");
  await capture("390,844", "opengeometry-demo-mobile.png");
  const dom = await dumpDom();
  if (!dom.includes('data-canvas-state="nonblank"')) {
    throw new Error("Canvas pixel check did not report a nonblank render.");
  }
  await readFile(resolve(artifactDir, "opengeometry-demo-desktop.png"));
  await readFile(resolve(artifactDir, "opengeometry-demo-mobile.png"));
  console.log("OpenGeometry smoke passed");
} finally {
  server.kill("SIGTERM");
}
