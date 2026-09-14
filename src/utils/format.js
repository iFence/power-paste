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

// 统一的体积展示：会话气泡、附件卡片与历史面板共用同一套换算。
export function formatBytes(size) {
  const value = Number(size || 0);
  if (value < 1000) {
    return `${value} B`;
  }
  if (value < 1_000_000) {
    return `${Math.round(value / 1000)} KB`;
  }
  if (value < 1_000_000_000) {
    return `${(value / 1_000_000).toFixed(1)} MB`;
  }
  return `${(value / 1_000_000_000).toFixed(2)} GB`;
}

// 会话气泡的时间标签：当天只显示时刻，昨天与更早补上日期，避免聊天流里出现完整时间戳。
export function formatMessageTime(timestampMs, locale = "en-US") {
  const value = Number(timestampMs || 0);
  if (!value) {
    return "";
  }

  const date = new Date(value);
  const now = new Date();
  const time = `${String(date.getHours()).padStart(2, "0")}:${String(
    date.getMinutes(),
  ).padStart(2, "0")}`;
  const isZh = locale === "zh-CN";

  const startOfToday = new Date(
    now.getFullYear(),
    now.getMonth(),
    now.getDate(),
  ).getTime();
  const startOfDay = new Date(
    date.getFullYear(),
    date.getMonth(),
    date.getDate(),
  ).getTime();
  const dayDiff = Math.round((startOfToday - startOfDay) / 86_400_000);

  if (dayDiff <= 0) {
    return time;
  }
  if (dayDiff === 1) {
    return isZh ? `昨天 ${time}` : `Yesterday ${time}`;
  }
  if (date.getFullYear() === now.getFullYear()) {
    return isZh
      ? `${date.getMonth() + 1}月${date.getDate()}日 ${time}`
      : `${date.toLocaleString("en-US", { month: "short" })} ${date.getDate()}, ${time}`;
  }
  return `${date.toLocaleDateString(isZh ? "zh-CN" : "en-US")} ${time}`;
}

// 会话消息流的日期分隔标题：今天 / 昨天 / 具体日期。
export function formatMessageDayLabel(timestampMs, locale = "en-US") {
  const value = Number(timestampMs || 0);
  if (!value) {
    return "";
  }

  const date = new Date(value);
  const now = new Date();
  const isZh = locale === "zh-CN";
  const startOfToday = new Date(
    now.getFullYear(),
    now.getMonth(),
    now.getDate(),
  ).getTime();
  const startOfDay = new Date(
    date.getFullYear(),
    date.getMonth(),
    date.getDate(),
  ).getTime();
  const dayDiff = Math.round((startOfToday - startOfDay) / 86_400_000);

  if (dayDiff <= 0) {
    return isZh ? "今天" : "Today";
  }
  if (dayDiff === 1) {
    return isZh ? "昨天" : "Yesterday";
  }
  if (date.getFullYear() === now.getFullYear()) {
    return isZh
      ? `${date.getMonth() + 1}月${date.getDate()}日`
      : `${date.toLocaleString("en-US", { month: "short" })} ${date.getDate()}`;
  }
  return date.toLocaleDateString(isZh ? "zh-CN" : "en-US");
}

export function truncate(text, max = 120) {
  if (text.length <= max) {
    return text;
  }

  return `${text.slice(0, max - 1)}…`;
}
