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

export function createPanelRouteSizing({ appWindow, persistence, isResizingRef }) {
  async function resizeTo(size) {
    await nextTick()
    await appWindow.setSize({
      type: 'Logical',
      width: size.width,
      height: size.height,
    })
    await new Promise((resolve) => setTimeout(resolve, 150))
  }

  async function applyRouteSize(routeName, oldRouteName) {
    if (isResizingRef.value || routeName === persistence.currentRoute()) {
      return
    }

    try {
      isResizingRef.value = true

      // 离开主面板时记住用户调整过的尺寸，供返回时恢复。
      if (oldRouteName === 'home') {
        await persistence.captureCurrentHomeSize(appWindow)
      }

      const fixedSize = WINDOW_SIZES[routeName]
      if (fixedSize) {
        await resizeTo(fixedSize)
        persistence.setCurrentRouteName(routeName)
        return
      }

      if (persistence.isMainLikeRoute(routeName)) {
        const savedHomeSize = persistence.savedSize()
        if (savedHomeSize) {
          await resizeTo(savedHomeSize)
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
