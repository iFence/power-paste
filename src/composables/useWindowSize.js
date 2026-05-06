import { getCurrentWindow } from '@tauri-apps/api/window'
import { watch, nextTick } from 'vue'
import { saveMainPanelSize } from '../services/tauriApi'

// 窗口尺寸配置
const WINDOW_SIZES = {
  settings: {
    width: 600,
    height: 500,
  },
}

// 保存用户在主面板的窗口尺寸（逻辑像素）
let savedHomeSize = null
let isResizing = false
let currentRouteName = null
let isFirstLoad = true
let unlistenResize = null

/**
 * 获取内容区逻辑尺寸，用于设置页进出时恢复主面板大小。
 */
async function getRestoreLogicalSize(appWindow) {
  const size = await appWindow.innerSize()
  if (size.type === 'Physical') {
    const scaleFactor = await appWindow.scaleFactor()
    return {
      width: Math.round(size.width / scaleFactor),
      height: Math.round(size.height / scaleFactor),
    }
  }
  return {
    width: size.width,
    height: size.height,
  }
}

async function getPersistedMainPanelSize(appWindow) {
  const size = await appWindow.outerSize()
  const scaleFactor = await appWindow.scaleFactor()
  if (size.type === 'Physical') {
    return {
      width: Math.round(size.width / scaleFactor),
      height: Math.round(size.height / scaleFactor),
    }
  }

  return {
    width: size.width,
    height: size.height,
  }
}

function isMainLikeRoute(routeName) {
  return routeName === 'home' || routeName === 'lanTransfer'
}

async function persistMainPanelSize(snapshot) {
  if (!snapshot) {
    return
  }

  try {
    await saveMainPanelSize({
      width: snapshot.width,
      height: snapshot.height,
    })
  } catch (error) {
    console.error('Failed to persist main panel size:', error)
  }
}

/**
 * 窗口尺寸管理 composable
 * 根据路由自动调整窗口尺寸
 * - 切换到设置面板：自动调整为固定尺寸（600x500）
 * - 切换回主面板：恢复用户之前调整的尺寸
 */
export function useWindowSize(route) {
  const appWindow = getCurrentWindow()

  if (!unlistenResize) {
    appWindow.onResized(async () => {
      if (isResizing || !isMainLikeRoute(currentRouteName)) {
        return
      }

      try {
        const size = await getRestoreLogicalSize(appWindow)
        savedHomeSize = size
        await persistMainPanelSize(await getPersistedMainPanelSize(appWindow))
      } catch (error) {
        console.error('Failed to track main panel size:', error)
      }
    }).then((unlisten) => {
      unlistenResize = unlisten
    }).catch((error) => {
      console.error('Failed to subscribe resize listener:', error)
    })
  }

  // 监听路由变化，自动调整窗口尺寸
  watch(
    () => route.name,
    async (routeName, oldRouteName) => {
      // 首次加载时，记录初始路由但不做调整
      if (isFirstLoad) {
        isFirstLoad = false
        currentRouteName = routeName
        
        // 如果首次加载就是主面板，保存初始尺寸
        if (isMainLikeRoute(routeName)) {
          try {
            savedHomeSize = await getRestoreLogicalSize(appWindow)
            await persistMainPanelSize(await getPersistedMainPanelSize(appWindow))
          } catch (error) {
            console.error('Failed to get initial size:', error)
          }
        }
        return
      }

      // 避免重复调整或相同路由
      if (isResizing || routeName === currentRouteName) {
        return
      }

      try {
        isResizing = true

        // 从主面板或互传面板切换到设置面板
        if (isMainLikeRoute(oldRouteName) && routeName === 'settings') {
          // 保存当前尺寸（逻辑像素）
          savedHomeSize = await getRestoreLogicalSize(appWindow)
          await persistMainPanelSize(await getPersistedMainPanelSize(appWindow))

          // 切换到设置面板的固定尺寸
          const targetSize = WINDOW_SIZES.settings

          await nextTick()

          await appWindow.setSize({
            type: 'Logical',
            width: targetSize.width,
            height: targetSize.height,
          })

          currentRouteName = routeName
          await new Promise((resolve) => setTimeout(resolve, 150))
        }
        // 从设置面板切换回主面板或互传面板
        else if (oldRouteName === 'settings' && isMainLikeRoute(routeName)) {
          // 恢复之前保存的尺寸
          if (savedHomeSize) {
            await nextTick()

            // 使用逻辑像素设置
            await appWindow.setSize({
              type: 'Logical',
              width: savedHomeSize.width,
              height: savedHomeSize.height,
            })

            currentRouteName = routeName
            await new Promise((resolve) => setTimeout(resolve, 150))
          } else {
            // 如果没有保存的尺寸，不做调整
            currentRouteName = routeName
          }
        }
        // 主面板和互传面板之间切换
        else {
          // 不做窗口尺寸调整，保持用户当前的窗口大小
          currentRouteName = routeName
        }
      } catch (error) {
        console.error('Failed to resize window:', error)
      } finally {
        isResizing = false
      }
    },
  )
}
