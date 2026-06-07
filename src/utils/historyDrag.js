function nonEmptyString(value) {
  return typeof value === 'string' && value.trim() ? value.trim() : ''
}

function pushDragData(items, type, value) {
  const text = nonEmptyString(value)
  if (!text || items.some((item) => item.type === type && item.value === text)) {
    return
  }

  items.push({ type, value: text })
}

export function buildHistoryDragData(item) {
  const data = []
  const kind = item?.kind
  const text = nonEmptyString(item?.fullText) || nonEmptyString(item?.preview)

  if (kind === 'link') {
    pushDragData(data, 'text/uri-list', text)
    pushDragData(data, 'text/plain', text)
    return data
  }

  if (kind === 'image') {
    return data
  }

  if (kind === 'mixed') {
    pushDragData(data, 'text/plain', text)
    return data
  }

  pushDragData(data, 'text/plain', text)
  return data
}

export function hasHistoryDragData(item) {
  if (nonEmptyString(item?.imageDataUrl)) {
    return true
  }

  return buildHistoryDragData(item).length > 0
}

function encodePathPart(value) {
  return encodeURIComponent(value).replaceAll('%2F', '/')
}

export function filePathToUri(path) {
  const value = nonEmptyString(path)
  if (!value) {
    return ''
  }

  if (/^[A-Za-z]:[\\/]/.test(value)) {
    return `file:///${encodePathPart(value.replaceAll('\\', '/'))}`
  }

  if (value.startsWith('/')) {
    return `file://${encodePathPart(value)}`
  }

  return `file://${encodePathPart(value.replaceAll('\\', '/'))}`
}
