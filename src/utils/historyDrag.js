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

export function imageDataUrlToDragFile(dataUrl, fileName = 'power-paste-image.png') {
  const value = nonEmptyString(dataUrl)
  const match = value.match(/^data:([^;,]+)?(;base64)?,(.*)$/)
  if (!match) {
    return null
  }

  const mimeType = match[1] || 'image/png'
  const isBase64 = Boolean(match[2])
  const body = match[3] || ''
  const binary = isBase64 ? atob(body) : decodeURIComponent(body)
  const bytes = new Uint8Array(binary.length)
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index)
  }

  return new File([bytes], fileName, { type: mimeType })
}
