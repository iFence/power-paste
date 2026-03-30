import http from "node:http";
import { spawn } from "node:child_process";

function readCliOption(name) {
  const index = process.argv.indexOf(name);
  if (index === -1) {
    return null;
  }
  return process.argv[index + 1] ?? null;
}

const DEV_HOST = readCliOption("--host") || process.env.POWER_PASTE_DEV_HOST || "127.0.0.1";
const DEV_PORT = Number(readCliOption("--port") || process.env.POWER_PASTE_DEV_PORT || "5173");
const DEV_URL = `http://${DEV_HOST}:${DEV_PORT}/`;
const CHECK_INTERVAL_MS = 500;
const START_TIMEOUT_MS = 120_000;

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
  console.log(`Reusing existing Vite dev server on ${DEV_HOST}:${DEV_PORT}`);
  process.exit(0);
}

function createDevCommand() {
  const packageManagerEntrypoint = process.env.npm_execpath;

  if (packageManagerEntrypoint) {
    return {
      command: process.execPath,
      args: [packageManagerEntrypoint, "dev"],
    };
  }

  if (process.platform === "win32") {
    return {
      command: "cmd.exe",
      args: ["/d", "/s", "/c", "pnpm dev"],
    };
  }

  return {
    command: "pnpm",
    args: ["dev"],
  };
}

const { command, args } = createDevCommand();
const child = spawn(command, args, {
  env: {
    ...process.env,
    POWER_PASTE_DEV_HOST: DEV_HOST,
    POWER_PASTE_DEV_PORT: String(DEV_PORT),
  },
  stdio: ["inherit", "pipe", "pipe"],
});

let sawPortInUse = false;
let childExited = false;

child.stdout?.on("data", (chunk) => {
  process.stdout.write(chunk);
});

child.stderr?.on("data", (chunk) => {
  const text = chunk.toString();
  if (text.includes(`Port ${DEV_PORT} is already in use`)) {
    sawPortInUse = true;
  }
  process.stderr.write(chunk);
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
  console.error(`Timed out waiting for the Vite dev server on ${DEV_HOST}:${DEV_PORT}`);
  process.exit(1);
}

if (sawPortInUse) {
  console.log(`Reusing existing Vite dev server after detecting port ${DEV_PORT} is already in use`);
  process.exit(0);
}

const exitCode = await new Promise((resolve, reject) => {
  child.on("exit", (code, signal) => {
    childExited = true;
    if (signal) {
      reject(new Error(`Vite dev server exited via signal ${signal}`));
      return;
    }
    resolve(code ?? 0);
  });
  child.on("error", reject);
});

if (childExited && sawPortInUse && (await isDevServerReady())) {
  console.log(`Reusing existing Vite dev server on ${DEV_HOST}:${DEV_PORT}`);
  process.exit(0);
}

process.exit(exitCode);
