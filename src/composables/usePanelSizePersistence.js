import { saveMainPanelSize } from '../services/tauriApi'

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

// 只有主面板（home）的尺寸需要被记住与恢复；设置页和 LocalSend 页使用固定尺寸。
function isMainLikeRoute(routeName) {
  return routeName === 'home'
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

export function createPanelSizePersistence() {
  let savedHomeSize = null
  let currentRouteName = null
  let unlistenResize = null

  async function captureCurrentHomeSize(appWindow) {
    savedHomeSize = await getRestoreLogicalSize(appWindow)
    await persistMainPanelSize(await getPersistedMainPanelSize(appWindow))
    return savedHomeSize
  }

  function setCurrentRouteName(routeName) {
    currentRouteName = routeName
  }

  function currentRoute() {
    return currentRouteName
  }

  function savedSize() {
    return savedHomeSize
  }

  function installResizeListener(appWindow, isResizingRef) {
    if (unlistenResize) {
      return
    }

    appWindow.onResized(async () => {
      if (isResizingRef.value || !isMainLikeRoute(currentRouteName)) {
        return
      }

      try {
        await captureCurrentHomeSize(appWindow)
      } catch (error) {
        console.error('Failed to track main panel size:', error)
      }
    }).then((unlisten) => {
      unlistenResize = unlisten
    }).catch((error) => {
      console.error('Failed to subscribe resize listener:', error)
    })
  }

  return {
    captureCurrentHomeSize,
    currentRoute,
    installResizeListener,
    isMainLikeRoute,
    savedSize,
    setCurrentRouteName,
  }
}
