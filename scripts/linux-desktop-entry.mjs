import fs from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

export const projectRoot = path.resolve(__dirname, '..')

// 应用标识（app id）：桌面门户按调用进程 cgroup 里 app-*.scope 的名字推导，
// GNOME 侧还会用 GLib 的 g_application_id_is_valid() 校验，要求反向域名格式
// （至少包含一个点，如 power-paste 会被直接丢弃）。因此这里必须与
// tauri.conf.json 的 identifier 保持一致，桌面项的文件名也必须与它同名，
// 桌面才能识别调用应用、并在应用列表里把它列成同一款应用。
export const APP_ID = 'com.yulei.powerpaste'

// 窗口 app-id 与 X11 WM_CLASS 取自可执行文件名：Tauri 未开启 enableGtkAppId
// 时 GTK 用程序名注册窗口，GNOME 再用桌面项的 StartupWMClass 把窗口关联到
// 应用图标，因此桌面项里的 StartupWMClass 必须保持可执行文件名。
export const WINDOW_CLASS = 'power-paste'

// 旧版本直接用可执行文件名当应用标识，保留它用于清理历史桌面项。
const LEGACY_APP_ID = WINDOW_CLASS

const ICON_FILE = path.join(projectRoot, 'src-tauri', 'icons', '128x128@2x.png')
const DEBUG_BINARY_PATH = path.join(projectRoot, 'src-tauri', 'target', 'debug', WINDOW_CLASS)
const RELEASE_BINARY_PATH = path.join(projectRoot, 'src-tauri', 'target', 'release', WINDOW_CLASS)

// 开发运行（pnpm tauri dev / cargo run）使用的可执行文件路径。
export const DEV_BINARY_PATH = DEBUG_BINARY_PATH

export async function isFile(targetPath) {
  try {
    return (await fs.stat(targetPath)).isFile()
  } catch {
    return false
  }
}

// 用户级桌面项目录，遵循 XDG 规范以便自定数据目录的环境也能生效。
export function applicationsDir(env = process.env) {
  const dataHome = env.XDG_DATA_HOME?.trim()
  const baseDir = dataHome || path.join(os.homedir(), '.local', 'share')
  return path.join(baseDir, 'applications')
}

// 安装路径未知时按构建产物推断：优先调试产物，其次发布产物。
export async function resolveBuiltBinary() {
  for (const candidate of [DEBUG_BINARY_PATH, RELEASE_BINARY_PATH]) {
    if (await isFile(candidate)) {
      return candidate
    }
  }

  throw new Error('未找到构建产物，请先运行 pnpm tauri dev 或 pnpm tauri build')
}

function desktopEntryContent(binaryPath) {
  return `[Desktop Entry]
Type=Application
Name=Power Paste
Comment=Native-feeling clipboard history manager (development build)
Exec="${binaryPath}"
Icon=${ICON_FILE}
StartupWMClass=${WINDOW_CLASS}
Terminal=false
Categories=Utility;
`
}

// 判断桌面项是否指向本项目构建产物：只清理我们自己写过的旧桌面项，
// 避免误删用户手写的同名文件。
function isProjectEntry(content) {
  const match = content.match(/^Exec="?([^"\n]+)"?$/m)
  if (!match) {
    return false
  }

  const binaryPath = match[1].trim()
  const suffix = path.join('target', 'debug', WINDOW_CLASS)
  const releaseSuffix = path.join('target', 'release', WINDOW_CLASS)
  return binaryPath.endsWith(suffix) || binaryPath.endsWith(releaseSuffix)
}

// 删除旧版用可执行文件名命名的桌面项，避免应用列表出现两个 Power Paste。
async function removeLegacyDesktopEntry(targetDir) {
  const legacyPath = path.join(targetDir, `${LEGACY_APP_ID}.desktop`)
  const content = await fs.readFile(legacyPath, 'utf8').catch(() => null)
  if (content === null || !isProjectEntry(content)) {
    return null
  }

  await fs.rm(legacyPath, { force: true })
  return legacyPath
}

// 写入（或更新）用户级桌面项，返回写入结果；内容已是最新时不重复写。
export async function ensureDesktopEntry({ binaryPath } = {}) {
  if (!(await isFile(ICON_FILE))) {
    throw new Error(`应用图标缺失：${ICON_FILE}`)
  }

  const targetBinary = binaryPath ?? (await resolveBuiltBinary())
  const targetDir = applicationsDir()
  const targetPath = path.join(targetDir, `${APP_ID}.desktop`)
  const content = desktopEntryContent(targetBinary)
  const removedLegacyPath = await removeLegacyDesktopEntry(targetDir)

  const existing = await fs.readFile(targetPath, 'utf8').catch(() => null)
  if (existing === content) {
    return { path: targetPath, binaryPath: targetBinary, written: false, removedLegacyPath }
  }

  await fs.mkdir(targetDir, { recursive: true })
  await fs.writeFile(targetPath, content, { mode: 0o644 })
  return { path: targetPath, binaryPath: targetBinary, written: true, removedLegacyPath }
}
