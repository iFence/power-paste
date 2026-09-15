import { ensureDesktopEntry } from './linux-desktop-entry.mjs'

// GNOME Wayland 不使用窗口自带的图标：任务切换器与应用列表按窗口 app-id（没有
// app-id 时退回 WM_CLASS）匹配 .desktop 文件；Wayland 下的全局快捷键门户也要求
// 进程具备与桌面项同名的应用标识，且该标识必须是反向域名格式（GNOME 用
// g_application_id_is_valid 校验，无点号的标识会被直接丢弃）。安装包会自带桌面项，
// 直接运行构建产物时没有，所以这里往用户目录写一份指向当前构建产物的桌面项。
async function main() {
  if (process.platform !== 'linux') {
    process.stdout.write('该命令只在 Linux 桌面环境需要，当前平台无需执行\n')
    return
  }

  const entry = await ensureDesktopEntry()
  process.stdout.write(`桌面项：${entry.path}\n`)
  process.stdout.write(`可执行文件：${entry.binaryPath}\n`)
  if (entry.removedLegacyPath) {
    process.stdout.write(`已清理旧桌面项：${entry.removedLegacyPath}\n`)
  }
  process.stdout.write(
    entry.written
      ? '已写入桌面项，请从应用列表（而不是终端）重新启动应用，桌面才会托管全局快捷键\n'
      : '桌面项已是最新，无需更新\n',
  )
}

main().catch((error) => {
  process.stderr.write(`${error.message}\n`)
  process.exitCode = 1
})
