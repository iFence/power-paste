export const HISTORY_TAG_COLORS = [
  'red',
  'orange',
  'yellow',
  'green',
  'blue',
  'purple',
  'gray',
]

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
