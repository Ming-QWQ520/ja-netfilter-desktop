/**
 * UI 全局状态：主题（system/light/dark）、语言（zh/en）、当前 Tab。
 * 持久化到 localStorage；主题默认跟随系统并监听系统切换。
 */

import { defineStore } from "pinia";
import { ref, watch } from "vue";

export type ThemeMode = "system" | "light" | "dark";
export type Locale = "zh" | "en";
export type TabId = "overview" | "config" | "logs" | "settings";

const THEME_KEY = "ja-netfilter-desktop:theme";
const LOCALE_KEY = "ja-netfilter-desktop:locale";

function systemPrefersDark(): boolean {
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

function loadTheme(): ThemeMode {
  const v = localStorage.getItem(THEME_KEY);
  return v === "light" || v === "dark" || v === "system" ? v : "system";
}

function loadLocale(): Locale {
  const v = localStorage.getItem(LOCALE_KEY);
  if (v === "zh" || v === "en") return v;
  // 默认跟随系统 UI 语言（ckey_script.ps1 同款逻辑）
  const lang = navigator.language.toLowerCase();
  return lang.startsWith("zh") ? "zh" : "en";
}

export const ui = defineStore("ui", () => {
  const theme = ref<ThemeMode>(loadTheme());
  const locale = ref<Locale>(loadLocale());
  const tab = ref<TabId>("overview");
  const isDark = ref(false);

  function applyTheme() {
    const dark =
      theme.value === "dark" || (theme.value === "system" && systemPrefersDark());
    isDark.value = dark;
    const root = document.documentElement;
    root.classList.toggle("md-dark", dark);
    root.classList.toggle("md-light", !dark);
    root.style.colorScheme = dark ? "dark" : "light";
  }

  // 系统主题切换监听
  window
    .matchMedia("(prefers-color-scheme: dark)")
    .addEventListener("change", () => {
      if (theme.value === "system") applyTheme();
    });

  watch(theme, (v) => {
    localStorage.setItem(THEME_KEY, v);
    applyTheme();
  });

  watch(locale, (v) => {
    localStorage.setItem(LOCALE_KEY, v);
  });

  applyTheme();

  function setTheme(mode: ThemeMode) {
    theme.value = mode;
  }
  function toggleTheme() {
    theme.value = isDark.value ? "light" : "dark";
  }
  function setLocale(l: Locale) {
    locale.value = l;
  }

  return {
    theme,
    locale,
    tab,
    isDark,
    setTheme,
    toggleTheme,
    setLocale,
  };
});
