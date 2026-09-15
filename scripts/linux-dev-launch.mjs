import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'

import { APP_ID, DEV_BINARY_PATH, ensureDesktopEntry } from './linux-desktop-entry.mjs'

const APP_SCOPE_PREFIX = 'app-dev'

// 桌面门户按调用进程 cgroup 里 app-*.scope 的名字确定应用标识（app id），
// 并要求存在同名 .desktop；从终端直接启动 dev 时两者都缺失，Wayland 下
// GlobalShortcuts 门户会以 “An app id is required” 拒绝绑定全局快捷键。
// 这里补齐开发用桌面项，并把 dev 进程树放进应用 scope。
export async function prepareLinuxDevLaunch(launch, env) {
  if (process.platform !== 'linux') {
    return launch
  }

  await ensureDevDesktopEntry()
  return (await resolveAppScopeCommand(launch, env)) ?? launch
}

// 写桌面项失败（例如用户数据目录只读）不应该阻塞开发流程。
async function ensureDevDesktopEntry() {
  try {
    const entry = await ensureDesktopEntry({ binaryPath: DEV_BINARY_PATH })
    if (entry.written) {
      console.log(`已写入开发用桌面项：${entry.path}`)
    }
  } catch (error) {
    console.warn(`跳过开发用桌面项：${error.message}`)
  }
}

// 非 systemd 发行版或受限环境下保持原有启动方式。
async function resolveAppScopeCommand(launch, env) {
  if (!env.XDG_RUNTIME_DIR) {
    return null
  }

  const systemdRun = findExecutable('systemd-run', env)
  if (!systemdRun || !canCreateAppScope(systemdRun, env)) {
    return null
  }

  // 桌面约定 scope 名为 app-<前缀>-<应用标识>-<唯一后缀>，门户用
  // ^app-(?:[[:alnum:]]+\-)?(.+?)(?:\-[[:alnum:]]*)(?:\.scope|\.slice)$ 取出应用标识，
  // 因此 "dev-" 前缀会被剥离，最终标识仍是 com.yulei.powerpaste。
  const unit = `${APP_SCOPE_PREFIX}-${APP_ID}-${process.pid}.scope`
  return {
    command: systemdRun,
    args: ['--user', '--scope', '--quiet', `--unit=${unit}`, '--', launch.command, ...launch.args],
  }
}

// 先建一个临时 scope 试探：systemd-run 存在但用户会话不可用时不能赌它成功，
// 否则整个 dev 流程会直接启动失败。
function canCreateAppScope(systemdRun, env) {
  try {
    execFileSync(
      systemdRun,
      [
        '--user',
        '--scope',
        '--quiet',
        `--unit=${APP_SCOPE_PREFIX}-check-${process.pid}.scope`,
        '--',
        'true',
      ],
      { stdio: 'ignore', env, timeout: 5000 },
    )
    return true
  } catch {
    return false
  }
}

function findExecutable(name, env) {
  const pathKey = Object.keys(env).find((key) => key.toUpperCase() === 'PATH') ?? 'PATH'

  for (const segment of (env[pathKey] ?? '').split(path.delimiter)) {
    if (!segment) {
      continue
    }

    const candidate = path.join(segment, name)
    if (isRegularFile(candidate)) {
      return candidate
    }
  }

  return null
}

function isRegularFile(targetPath) {
  try {
    return fs.statSync(targetPath).isFile()
  } catch {
    return false
  }
}
