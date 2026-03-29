import http from "node:http";
import { spawn } from "node:child_process";

const DEV_URL = "http://127.0.0.1:5173/";
const CHECK_INTERVAL_MS = 500;
const START_TIMEOUT_MS = 30_000;

function isDevServerReady() {
  return new Promise((resolve) => {
    const request = http.get(DEV_URL, (response) => {
      response.resume();
      resolve(response.statusCode >= 200 && response.statusCode < 500);
    });

    request.on("error", () => resolve(false));
    request.setTimeout(1000, () => {
      request.destroy();
      resolve(false);
    });
  });
}

async function waitForServerReady(timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await isDevServerReady()) {
      return true;
    }
    await new Promise((resolve) => setTimeout(resolve, CHECK_INTERVAL_MS));
  }
  return false;
}

if (await isDevServerReady()) {
  console.log("Reusing existing Vite dev server on 127.0.0.1:5173");
  process.exit(0);
}

const command = process.platform === "win32" ? "pnpm.cmd" : "pnpm";
const child = spawn(command, ["dev"], {
  stdio: "inherit",
});

const terminateChild = (signal) => {
  if (!child.killed) {
    child.kill(signal);
  }
};

process.on("SIGINT", () => terminateChild("SIGINT"));
process.on("SIGTERM", () => terminateChild("SIGTERM"));
process.on("exit", () => terminateChild("SIGTERM"));

const ready = await waitForServerReady(START_TIMEOUT_MS);
if (!ready) {
  terminateChild("SIGTERM");
  console.error("Timed out waiting for the Vite dev server on 127.0.0.1:5173");
  process.exit(1);
}

const exitCode = await new Promise((resolve, reject) => {
  child.on("exit", (code, signal) => {
    if (signal) {
      reject(new Error(`Vite dev server exited via signal ${signal}`));
      return;
    }
    resolve(code ?? 0);
  });
  child.on("error", reject);
});

process.exit(exitCode);
