import fs from "node:fs/promises";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const projectRoot = path.resolve(__dirname, "..");
const tauriCliPath = path.join(projectRoot, "node_modules", "@tauri-apps", "cli", "tauri.js");
const tauriConfigPath = path.join(projectRoot, "src-tauri", "tauri.conf.json");
const DEFAULT_DEV_HOST = "127.0.0.1";
const DEFAULT_DEV_PORT = 5173;

async function readWindowConfigWithDevtools() {
  const raw = await fs.readFile(tauriConfigPath, "utf8");
  const config = JSON.parse(raw);
  const windows = Array.isArray(config.app?.windows) ? config.app.windows : [];

  return windows.map((window) => ({
    ...window,
    devtools: true,
  }));
}

function parsePort(value, fallback) {
  const numeric = Number(value);
  if (!Number.isInteger(numeric) || numeric <= 0 || numeric > 65535) {
    return fallback;
  }
  return numeric;
}

function canListen(host, port) {
  return new Promise((resolve) => {
    const server = net.createServer();

    server.once("error", () => resolve(false));
    server.once("listening", () => {
      server.close(() => resolve(true));
    });
    server.listen(port, host);
  });
}

async function findAvailablePort(host, preferredPort) {
  for (let port = preferredPort; port < preferredPort + 50; port += 1) {
    if (await canListen(host, port)) {
      return port;
    }
  }

  return new Promise((resolve, reject) => {
    const server = net.createServer();

    server.once("error", reject);
    server.once("listening", () => {
      const address = server.address();
      if (!address || typeof address === "string") {
        server.close(() => reject(new Error("Failed to determine dynamic dev port")));
        return;
      }
      server.close(() => resolve(address.port));
    });

    server.listen(0, host);
  });
}

async function main() {
  const args = process.argv.slice(2);
  const isDevCommand = args[0] === "dev";
  const env = { ...process.env };
  let tauriArgs = args;
  const windows = await readWindowConfigWithDevtools();

  if (isDevCommand) {
    const host = env.POWER_PASTE_DEV_HOST || DEFAULT_DEV_HOST;
    const preferredPort = parsePort(env.POWER_PASTE_DEV_PORT, DEFAULT_DEV_PORT);
    const port = await findAvailablePort(host, preferredPort);
    const devUrl = `http://${host}:${port}`;
    tauriArgs = [
      "dev",
      "--config",
      JSON.stringify({
        build: {
          beforeDevCommand: {
            script: `node scripts/tauri-dev-server.mjs --host ${host} --port ${port}`,
            wait: false,
          },
          devUrl,
        },
        app: {
          windows,
        },
      }),
      ...args.slice(1),
    ];
  } else if (args[0] === "build") {
    tauriArgs = [
      "build",
      "--config",
      JSON.stringify({
        app: {
          windows,
        },
      }),
      ...args.slice(1),
    ];
  }

  const child = spawn(process.execPath, [tauriCliPath, ...tauriArgs], {
    cwd: projectRoot,
    env,
    stdio: "inherit",
  });

  child.on("error", (error) => {
    console.error(error);
    process.exit(1);
  });

  child.on("exit", (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
      return;
    }
    process.exit(code ?? 0);
  });
}

await main();
