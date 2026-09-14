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

// Wayland 会话下全局快捷键由桌面门户托管，绑定结果以错误码形式回传，
// 这里把错误码映射成可操作的提示文案；非门户错误返回空串，由调用方
// 继续按“快捷键冲突”处理。
export function shortcutIssueMessage(code, t) {
  if (!code) {
    return '';
  }

  if (code.startsWith('wayland_portal_unavailable')) {
    return t('waylandPortalUnavailable');
  }
  if (code.startsWith('wayland_portal_denied')) {
    return t('waylandPortalDenied');
  }
  if (code.startsWith('wayland_portal_closed')) {
    return t('waylandPortalClosed');
  }
  if (code.startsWith('wayland_portal_no_app_id')) {
    return t('waylandPortalNoAppId');
  }
  if (code.startsWith('wayland_portal_failed')) {
    return t('waylandPortalFailed');
  }

  return '';
}
