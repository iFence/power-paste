import { nextTick } from 'vue'

const WINDOW_SIZES = {
  settings: {
    width: 650,
    height: 500,
  },
  // 互传页使用宽屏桌面布局，窄窗时由页面内部切换为底部导航。
  lanTransfer: {
    width: 900,
    height: 600,
  },
}

// 主面板的最小尺寸，返回主页时恢复，避免主面板被缩得过小。
const HOME_MIN_SIZE = {
  width: 380,
  height: 600,
}

export function createPanelRouteSizing({ appWindow, persistence, isResizingRef }) {
  async function setSize(size) {
    await nextTick()
    await appWindow.setSize({
      type: 'Logical',
      width: size.width,
      height: size.height,
    })
    await new Promise((resolve) => setTimeout(resolve, 150))
  }

  // macOS 上 setContentSize 受 contentMinSize 约束，设置页（高度 500）低于
  // 主面板最小高度 600 时会被强制拉高，导致与 Windows 表现不一致。进入
  // 设置页 / 互传页前先放宽最小尺寸，返回主页时恢复。放宽最小尺寸属于
  // 尽力而为，失败不阻断核心的 setSize 流程。
  async function relaxMinSizeForFixedRoute(size) {
    try {
      await appWindow.setMinSize({
        type: 'Logical',
        width: Math.min(size.width, HOME_MIN_SIZE.width),
        height: Math.min(size.height, HOME_MIN_SIZE.height),
      })
    } catch (error) {
      console.error('Failed to relax min size:', error)
    }
  }

  async function restoreHomeMinSize() {
    try {
      await appWindow.setMinSize({
        type: 'Logical',
        width: HOME_MIN_SIZE.width,
        height: HOME_MIN_SIZE.height,
      })
    } catch (error) {
      console.error('Failed to restore min size:', error)
    }
  }

  async function applyRouteSize(routeName, oldRouteName) {
    if (isResizingRef.value || routeName === persistence.currentRoute()) {
      return
    }

    try {
      isResizingRef.value = true

      // 离开主面板时记住用户调整过的尺寸，供返回时恢复。该步骤属于
      // 尽力而为，失败不应阻断后续的窗口尺寸调整。
      if (oldRouteName === 'home') {
        try {
          await persistence.captureCurrentHomeSize(appWindow)
        } catch (error) {
          console.error('Failed to capture home size:', error)
        }
      }

      const fixedSize = WINDOW_SIZES[routeName]
      if (fixedSize) {
        await relaxMinSizeForFixedRoute(fixedSize)
        await setSize(fixedSize)
        persistence.setCurrentRouteName(routeName)
        return
      }

      if (persistence.isMainLikeRoute(routeName)) {
        // 回到主面板：先恢复最小尺寸约束，再恢复到用户上次调整的尺寸。
        await restoreHomeMinSize()

        const savedHomeSize = persistence.savedSize()
        if (savedHomeSize) {
          await setSize(savedHomeSize)
          persistence.setCurrentRouteName(routeName)
          return
        }
      }

      persistence.setCurrentRouteName(routeName)
    } catch (error) {
      console.error('Failed to resize window:', error)
    } finally {
      isResizingRef.value = false
    }
  }

  return {
    applyRouteSize,
  }
}
