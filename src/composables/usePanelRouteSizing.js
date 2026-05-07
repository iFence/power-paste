import { nextTick } from 'vue'

const WINDOW_SIZES = {
  settings: {
    width: 600,
    height: 500,
  },
}

export function createPanelRouteSizing({ appWindow, persistence, isResizingRef }) {
  async function applyRouteSize(routeName, oldRouteName) {
    if (isResizingRef.value || routeName === persistence.currentRoute()) {
      return
    }

    try {
      isResizingRef.value = true

      if (persistence.isMainLikeRoute(oldRouteName) && routeName === 'settings') {
        await persistence.captureCurrentHomeSize(appWindow)
        const targetSize = WINDOW_SIZES.settings

        await nextTick()
        await appWindow.setSize({
          type: 'Logical',
          width: targetSize.width,
          height: targetSize.height,
        })

        persistence.setCurrentRouteName(routeName)
        await new Promise((resolve) => setTimeout(resolve, 150))
        return
      }

      if (oldRouteName === 'settings' && persistence.isMainLikeRoute(routeName)) {
        const savedHomeSize = persistence.savedSize()
        if (savedHomeSize) {
          await nextTick()
          await appWindow.setSize({
            type: 'Logical',
            width: savedHomeSize.width,
            height: savedHomeSize.height,
          })

          persistence.setCurrentRouteName(routeName)
          await new Promise((resolve) => setTimeout(resolve, 150))
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
