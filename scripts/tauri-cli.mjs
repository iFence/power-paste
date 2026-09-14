import fs from "node:fs/promises";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";
import { prepareLinuxDevLaunch } from "./linux-dev-launch.mjs";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const projectRoot = path.resolve(__dirname, "..");
const tauriCliPath = path.join(projectRoot, "node_modules", "@tauri-apps", "cli", "tauri.js");
const tauriConfigPath = path.join(projectRoot, "src-tauri", "tauri.conf.json");
const DEFAULT_DEV_HOST = "127.0.0.1";
const DEFAULT_DEV_PORT = 5173;
const CARGO_EXECUTABLE = process.platform === "win32" ? "cargo.exe" : "cargo";

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

// 判断路径是否为真实存在的文件，用于探测 cargo 可执行文件
async function isExistingFile(targetPath) {
  try {
    const stats = await fs.stat(targetPath);
    return stats.isFile();
  } catch {
    return false;
  }
}

// 取环境变量中表示 PATH 的键名，Windows 下进程环境里可能写成 Path
function getPathKey(env) {
  const key = Object.keys(env).find((item) => item.toUpperCase() === "PATH");
  return key ?? "PATH";
}

// 在当前 PATH 中查找 cargo，命中说明环境已就绪，无需补齐
async function findCargoInPath(env) {
  const pathKey = getPathKey(env);
  const segments = (env[pathKey] ?? "").split(path.delimiter).filter(Boolean);

  for (const segment of segments) {
    const candidate = path.join(segment, CARGO_EXECUTABLE);
    if (await isExistingFile(candidate)) {
      return candidate;
    }
  }

  return null;
}

// 依次参考 CARGO、CARGO_HOME、~/.cargo/bin 推断 rustup 的安装目录
async function resolveCargoDir(env) {
  if (env.CARGO && (await isExistingFile(env.CARGO))) {
    return path.dirname(env.CARGO);
  }

  const candidates = [];
  if (env.CARGO_HOME) {
    candidates.push(path.join(env.CARGO_HOME, "bin"), env.CARGO_HOME);
  }
  candidates.push(path.join(os.homedir(), ".cargo", "bin"));

  for (const dir of candidates) {
    if (await isExistingFile(path.join(dir, CARGO_EXECUTABLE))) {
      return dir;
    }
  }

  return null;
}

// 保证 Tauri CLI 能调用 cargo：GUI 启动、IDE 任务或安装 Rust 后未重开的终端
// 可能没加载 shell 配置，PATH 中缺少 ~/.cargo/bin，此时 CLI 只会报
// "failed to get cargo metadata: No such file or directory"，缺少可操作信息。
async function ensureCargoOnPath(env) {
  if (await findCargoInPath(env)) {
    return;
  }

  const cargoDir = await resolveCargoDir(env);
  if (!cargoDir) {
    console.error(
      `未找到 ${CARGO_EXECUTABLE}：请先安装 Rust 工具链（https://rustup.rs），或将其所在目录加入 PATH。`
    );
    return;
  }

  const pathKey = getPathKey(env);
  env[pathKey] = `${cargoDir}${path.delimiter}${env[pathKey] ?? ""}`;
  console.log(`Rust 工具链不在 PATH 中，已自动追加：${cargoDir}`);
}

async function main() {
  const args = process.argv.slice(2);
  const isDevCommand = args[0] === "dev";
  const env = { ...process.env };
  let tauriArgs = args;
  const windows = await readWindowConfigWithDevtools();

  await ensureCargoOnPath(env);

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
    tauriArgs = ["build", "--features", "custom-protocol", ...args.slice(1)];
  }

  const launch = {
    command: process.execPath,
    args: [tauriCliPath, ...tauriArgs],
  };
  // Linux 下把 dev 进程树放进应用 scope，桌面门户才能识别应用标识并托管全局快捷键。
  const devLaunch = isDevCommand ? await prepareLinuxDevLaunch(launch, env) : launch;

  const child = spawn(devLaunch.command, devLaunch.args, {
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
