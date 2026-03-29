import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { applyTheme as syncTheme } from "../utils/theme";

export function useTheme({ currentThemeMode, currentAccentColor }) {
  const systemPrefersDark = ref(false);
  let removeThemeListener = null;

  const resolvedTheme = computed(() =>
    currentThemeMode.value === "system"
      ? systemPrefersDark.value
        ? "dark"
        : "light"
      : currentThemeMode.value,
  );

  watch([resolvedTheme, currentAccentColor], () => {
    syncTheme(resolvedTheme.value, currentAccentColor.value);
  });

  onMounted(() => {
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    systemPrefersDark.value = mediaQuery.matches;
    const handleThemeChange = (event) => {
      systemPrefersDark.value = event.matches;
    };

    if (typeof mediaQuery.addEventListener === "function") {
      mediaQuery.addEventListener("change", handleThemeChange);
      removeThemeListener = () => mediaQuery.removeEventListener("change", handleThemeChange);
    } else {
      mediaQuery.addListener(handleThemeChange);
      removeThemeListener = () => mediaQuery.removeListener(handleThemeChange);
    }

    syncTheme(resolvedTheme.value, currentAccentColor.value);
  });

  onUnmounted(() => {
    removeThemeListener?.();
  });

  return {
    resolvedTheme,
    systemPrefersDark,
  };
}
