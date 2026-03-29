export function formatRelativeTime(iso, locale = "en-US") {
  const timestamp = new Date(iso).getTime();
  const diff = Date.now() - timestamp;
  const minute = 60_000;
  const hour = minute * 60;
  const day = hour * 24;
  const isZh = locale === "zh-CN";

  if (diff < minute) {
    return isZh ? "刚刚" : "Just now";
  }

  if (diff < hour) {
    return isZh ? `${Math.floor(diff / minute)} 分钟前` : `${Math.floor(diff / minute)}m ago`;
  }

  if (diff < day) {
    return isZh ? `${Math.floor(diff / hour)} 小时前` : `${Math.floor(diff / hour)}h ago`;
  }

  return isZh ? `${Math.floor(diff / day)} 天前` : `${Math.floor(diff / day)}d ago`;
}

export function truncate(text, max = 120) {
  if (text.length <= max) {
    return text;
  }

  return `${text.slice(0, max - 1)}…`;
}
