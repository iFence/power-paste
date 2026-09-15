export const HISTORY_TAG_COLORS = [
  'red',
  'orange',
  'yellow',
  'green',
  'blue',
  'purple',
  'gray',
]

// 门户托管快捷键（Linux Wayland）下，桌面只报告“快捷键失活”，不会说明修饰键是否
// 仍被按住：面板能收到键盘事件时，等修饰键松开再提交，这里只作为异常兜底时限；
// 桌面把按键完全吞掉（收不到任何按键事件）时，退回到“失活后延迟提交”，这段延迟
// 同时充当连续敲击切换候选项的时间窗口，取值需覆盖正常敲击节奏。
export const QUICK_PASTE_HOLD_TIMEOUT_MS = 800
export const QUICK_PASTE_RELEASE_FALLBACK_MS = 400

export function createEmptyTagLabels() {
  return Object.fromEntries(HISTORY_TAG_COLORS.map((color) => [color, '']))
}

export function normalizeTagLabels(input) {
  const next = createEmptyTagLabels()
  if (!input || typeof input !== 'object') {
    return next
  }

  for (const color of HISTORY_TAG_COLORS) {
    const value = input[color]
    next[color] = typeof value === 'string' ? value : ''
  }

  return next
}

export function defaultTagLabelKey(color) {
  return `tagDefaultName${color[0].toUpperCase()}${color.slice(1)}`
}

export function resolveTagLabel(color, tagLabels, t) {
  const customLabel = tagLabels?.[color]
  if (typeof customLabel === 'string' && customLabel.trim()) {
    return customLabel.trim()
  }

  return t(defaultTagLabelKey(color))
}
