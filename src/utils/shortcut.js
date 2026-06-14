function normalizeModifierKey(key, platform = 'unknown') {
  const lower = key.toLowerCase();

  if (lower === 'control') {
    return 'Ctrl';
  }
  if (lower === 'meta') {
    return platform === 'macos' ? 'Command' : 'Super';
  }
  if (lower === 'alt') {
    return 'Alt';
  }
  if (lower === 'shift') {
    return 'Shift';
  }

  return '';
}

export function normalizeShortcutKey(key, platform = 'unknown') {
  if (!key) {
    return '';
  }

  const lower = key.toLowerCase();
  if (lower === 'backquote' || lower === '`') {
    return '`';
  }
  if (lower === ' ') {
    return 'Space';
  }
  if (lower === 'arrowup') {
    return 'Up';
  }
  if (lower === 'arrowdown') {
    return 'Down';
  }
  if (lower === 'arrowleft') {
    return 'Left';
  }
  if (lower === 'arrowright') {
    return 'Right';
  }
  if (lower === 'escape') {
    return 'Esc';
  }

  const modifierKey = normalizeModifierKey(key, platform);
  if (modifierKey) {
    return modifierKey;
  }
  if (lower.length === 1) {
    return lower.toUpperCase();
  }
  return key[0].toUpperCase() + key.slice(1);
}

export function normalizeShortcutValue(shortcut, platform = 'unknown') {
  if (!shortcut) {
    return '';
  }

  return shortcut
    .split('+')
    .map((token) => normalizeShortcutKey(token.trim(), platform))
    .filter(Boolean)
    .join('+');
}
