export function normalizeShortcutKey(key) {
  if (!key) {
    return "";
  }

  const lower = key.toLowerCase();
  if (lower === " ") {
    return "Space";
  }
  if (lower === "arrowup") {
    return "Up";
  }
  if (lower === "arrowdown") {
    return "Down";
  }
  if (lower === "arrowleft") {
    return "Left";
  }
  if (lower === "arrowright") {
    return "Right";
  }
  if (lower === "escape") {
    return "Esc";
  }
  if (lower === "control") {
    return "Ctrl";
  }
  if (lower === "meta") {
    return "Meta";
  }
  if (lower === "alt") {
    return "Alt";
  }
  if (lower === "shift") {
    return "Shift";
  }
  if (lower.length === 1) {
    return lower.toUpperCase();
  }
  return key[0].toUpperCase() + key.slice(1);
}
