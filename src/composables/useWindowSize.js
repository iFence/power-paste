import { getCurrentWindow } from '@tauri-apps/api/window'
import { watch, nextTick } from 'vue'

// 窗口尺寸配置
const WINDOW_SIZES = {
  settings: {
    width: 600,
    height: 500,
  },
}

// 保存用户在主面板的窗口尺寸
let savedHomeSize = null
let isResizing = false
let currentRouteName = null
let isFirstLoad = true

/**
 * 窗口尺寸管理 composable
 * 根据路由自动调整窗口尺寸
 * - 切换到设置面板：自动调整为固定尺寸（600x500）
 * - 切换回主面板：恢复用户之前调整的尺寸
 */
export function useWindowSize(route) {
  const appWindow = getCurrentWindow()

  // 监听路由变化，自动调整窗口尺寸
  watch(
    () => route.name,
    async (routeName, oldRouteName) => {
      // 首次加载时，记录初始路由但不做调整
      if (isFirstLoad) {
        isFirstLoad = false
        currentRouteName = routeName
        
        // 如果首次加载就是主面板，保存初始尺寸
        if (routeName === 'home' || routeName === 'lanTransfer') {
          try {
            const currentSize = await appWindow.innerSize()
            savedHomeSize = {
              width: currentSize.width,
              height: currentSize.height,
            }
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
        if ((oldRouteName === 'home' || oldRouteName === 'lanTransfer') && routeName === 'settings') {
          // 保存当前尺寸（在切换前保存）
          const currentSize = await appWindow.innerSize()
          savedHomeSize = {
            width: currentSize.width,
            height: currentSize.height,
          }

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
        else if (oldRouteName === 'settings' && (routeName === 'home' || routeName === 'lanTransfer')) {
          // 恢复之前保存的尺寸
          if (savedHomeSize) {
            await nextTick()

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
