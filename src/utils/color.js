function isByteValue(value) {
  if (!/^\d{1,3}$/.test(value)) {
    return false
  }

  const next = Number(value)
  return Number.isInteger(next) && next >= 0 && next <= 255
}

function isAlphaValue(value) {
  if (!/^\d*\.?\d+$/.test(value)) {
    return false
  }

  const next = Number(value)
  return Number.isFinite(next) && next >= 0 && next <= 1
}

function isHexColor(value) {
  return /^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/.test(value)
}

function isRgbColor(value) {
  const match = /^rgb\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})\s*\)$/i.exec(value)
  if (!match) {
    return false
  }

  return match.slice(1).every(isByteValue)
}

function isRgbaColor(value) {
  const match =
    /^rgba\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d*\.?\d+)\s*\)$/i.exec(value)
  if (!match) {
    return false
  }

  return match.slice(1, 4).every(isByteValue) && isAlphaValue(match[4])
}

function isHueValue(value) {
  if (!/^-?\d*\.?\d+$/.test(value)) {
    return false
  }

  return Number.isFinite(Number(value))
}

function isPercentageValue(value) {
  if (!/^\d*\.?\d+%$/.test(value)) {
    return false
  }

  const next = Number(value.slice(0, -1))
  return Number.isFinite(next) && next >= 0 && next <= 100
}

function isHslColor(value) {
  const match =
    /^hsl\(\s*(-?\d*\.?\d+)\s*,\s*(\d*\.?\d+%)\s*,\s*(\d*\.?\d+%)\s*\)$/i.exec(value)
  if (!match) {
    return false
  }

  return isHueValue(match[1]) && isPercentageValue(match[2]) && isPercentageValue(match[3])
}

function isHslaColor(value) {
  const match =
    /^hsla\(\s*(-?\d*\.?\d+)\s*,\s*(\d*\.?\d+%)\s*,\s*(\d*\.?\d+%)\s*,\s*(\d*\.?\d+)\s*\)$/i.exec(value)
  if (!match) {
    return false
  }

  return (
    isHueValue(match[1]) &&
    isPercentageValue(match[2]) &&
    isPercentageValue(match[3]) &&
    isAlphaValue(match[4])
  )
}

export function resolvePreviewColor(text) {
  if (typeof text !== 'string') {
    return null
  }

  const normalized = text.trim()
  if (!normalized || normalized.includes('\n')) {
    return null
  }

  if (
    isHexColor(normalized) ||
    isRgbColor(normalized) ||
    isRgbaColor(normalized) ||
    isHslColor(normalized) ||
    isHslaColor(normalized)
  ) {
    return normalized
  }

  return null
}
