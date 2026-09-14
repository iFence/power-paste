import fs from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

export const projectRoot = path.resolve(__dirname, '..')

// 窗口 app-id、X11 的 WM_CLASS 与桌面门户的应用标识都取自可执行文件名，
// 桌面项的文件名与 StartupWMClass 必须与之一致：GNOME Wayland 才能把窗口
// 与应用图标关联起来，GlobalShortcuts 门户才能识别调用应用。
export const APP_ID = 'power-paste'

const ICON_FILE = path.join(projectRoot, 'src-tauri', 'icons', '128x128@2x.png')
const DEBUG_BINARY_PATH = path.join(projectRoot, 'src-tauri', 'target', 'debug', APP_ID)
const RELEASE_BINARY_PATH = path.join(projectRoot, 'src-tauri', 'target', 'release', APP_ID)

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
StartupWMClass=${APP_ID}
Terminal=false
Categories=Utility;
`
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

  const existing = await fs.readFile(targetPath, 'utf8').catch(() => null)
  if (existing === content) {
    return { path: targetPath, binaryPath: targetBinary, written: false }
  }

  await fs.mkdir(targetDir, { recursive: true })
  await fs.writeFile(targetPath, content, { mode: 0o644 })
  return { path: targetPath, binaryPath: targetBinary, written: true }
}
