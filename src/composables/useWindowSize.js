import { getCurrentWindow } from '@tauri-apps/api/window'
import { ref, watch } from 'vue'
import { createPanelRouteSizing } from './usePanelRouteSizing'
import { createPanelSizePersistence } from './usePanelSizePersistence'

/**
 * 窗口尺寸管理 composable
 * 根据路由自动调整窗口尺寸
 * - 切换到设置面板：自动调整为固定尺寸（600x500）
 * - 切换回主面板：恢复用户之前调整的尺寸
 */
export function useWindowSize(route) {
  const appWindow = getCurrentWindow()
  const isResizing = ref(false)
  const persistence = createPanelSizePersistence()
  const routeSizing = createPanelRouteSizing({
    appWindow,
    persistence,
    isResizingRef: isResizing,
  })
  let isFirstLoad = true

  persistence.installResizeListener(appWindow, isResizing)

  // 监听路由变化，自动调整窗口尺寸
  watch(
    () => route.name,
    async (routeName, oldRouteName) => {
      // 首次加载时，记录初始路由但不做调整
      if (isFirstLoad) {
        isFirstLoad = false
        persistence.setCurrentRouteName(routeName)
        
        // 如果首次加载就是主面板，保存初始尺寸
        if (persistence.isMainLikeRoute(routeName)) {
          try {
            await persistence.captureCurrentHomeSize(appWindow)
          } catch (error) {
            console.error('Failed to get initial size:', error)
          }
        }
        return
      }

      await routeSizing.applyRouteSize(routeName, oldRouteName)
    },
  )
}
