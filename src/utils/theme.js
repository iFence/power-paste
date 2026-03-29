export const accentPalettes = {
  amber: {
    primary: "#f0b35f",
    primaryStrong: "#dd8648",
    primarySoft: "rgba(240, 179, 95, 0.2)",
    primaryText: "#24160d",
    primaryGlow: "rgba(240, 179, 95, 0.36)",
  },
  ocean: {
    primary: "#68b6ff",
    primaryStrong: "#3e7fe6",
    primarySoft: "rgba(104, 182, 255, 0.2)",
    primaryText: "#0d1a2a",
    primaryGlow: "rgba(104, 182, 255, 0.32)",
  },
  jade: {
    primary: "#62d6b1",
    primaryStrong: "#2f9f83",
    primarySoft: "rgba(98, 214, 177, 0.2)",
    primaryText: "#0c1f1b",
    primaryGlow: "rgba(98, 214, 177, 0.3)",
  },
  rose: {
    primary: "#f08db0",
    primaryStrong: "#d45a86",
    primarySoft: "rgba(240, 141, 176, 0.2)",
    primaryText: "#2b1019",
    primaryGlow: "rgba(240, 141, 176, 0.32)",
  },
};

function hexToRgba(hex, alpha) {
  const normalized = hex.replace("#", "");
  if (normalized.length !== 6) {
    return hex;
  }

  const red = Number.parseInt(normalized.slice(0, 2), 16);
  const green = Number.parseInt(normalized.slice(2, 4), 16);
  const blue = Number.parseInt(normalized.slice(4, 6), 16);
  return `rgba(${red}, ${green}, ${blue}, ${alpha})`;
}

export function applyTheme(theme, accentColor) {
  const root = document.documentElement;
  const palette = accentPalettes[accentColor] || accentPalettes.amber;

  root.dataset.theme = theme;
  root.style.setProperty("--accent-primary", palette.primary);
  root.style.setProperty("--accent-primary-strong", palette.primaryStrong);
  root.style.setProperty("--accent-primary-soft", palette.primarySoft);
  root.style.setProperty("--accent-primary-hover-soft", hexToRgba(palette.primary, 0.18));
  root.style.setProperty("--accent-primary-text", palette.primaryText);
  root.style.setProperty("--accent-primary-glow", palette.primaryGlow);
}
