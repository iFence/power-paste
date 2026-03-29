export function escapeHtml(text) {
  return text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

export function looksLikeCode(text) {
  if (!text) {
    return false;
  }

  const sample = text.slice(0, 400);
  const lines = sample.split("\n");
  const indentedLines = lines.filter((line) => /^\s{2,}\S/.test(line)).length;
  const signalMatches =
    (sample.match(/(const |let |function |return |import |export |class |def |SELECT |FROM |WHERE |<\w+|=>|::|#include|fn )/g) || [])
      .length;

  return sample.includes("\n") && (indentedLines >= 2 || signalMatches >= 2);
}

export function highlightCode(text) {
  const escaped = escapeHtml(text);
  return escaped
    .replace(/("(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'|`(?:[^`\\]|\\.)*`)/g, '<span class="code-string">$1</span>')
    .replace(/\b(const|let|var|function|return|import|export|from|if|else|for|while|switch|case|break|continue|class|extends|new|try|catch|finally|async|await|def|class|fn|pub|impl|struct|enum|match|SELECT|FROM|WHERE|INSERT|UPDATE|DELETE|CREATE|ALTER|DROP)\b/g, '<span class="code-keyword">$1</span>')
    .replace(/(\/\/.*|#.*|\/\*[\s\S]*?\*\/)/g, '<span class="code-comment">$1</span>');
}

export function previewHtml(item) {
  if (!item.fullText) {
    return "";
  }

  const text = item.fullText ?? item.preview ?? "";
  if (!looksLikeCode(text)) {
    return "";
  }

  return highlightCode(text.split("\n").slice(0, 5).join("\n"));
}
