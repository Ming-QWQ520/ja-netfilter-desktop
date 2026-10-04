<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";

import { useSettingsStore } from "@/stores/settings";

const route = useRoute();
const settings = useSettingsStore();

const nav = [
  { to: "/", label: "总览", icon: "grid" },
  { to: "/configs", label: "插件配置", icon: "sliders" },
  { to: "/vmoptions", label: "vmoptions", icon: "code" },
  { to: "/plugins", label: "插件", icon: "puzzle" },
  { to: "/logs", label: "日志", icon: "terminal" },
  { to: "/settings", label: "设置", icon: "cog" },
];

const version = computed(() => settings.info?.app_version ?? "0.1.0");
const jarOk = computed(() => settings.info?.jar_exists ?? false);

function iconSvg(name: string) {
  switch (name) {
    case "grid":
      return `<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>`;
    case "sliders":
      return `<line x1="4" y1="6" x2="20" y2="6"/><line x1="4" y1="12" x2="20" y2="12"/><line x1="4" y1="18" x2="20" y2="18"/><circle cx="9" cy="6" r="2" fill="var(--bg-0)"/><circle cx="15" cy="12" r="2" fill="var(--bg-0)"/><circle cx="9" cy="18" r="2" fill="var(--bg-0)"/>`;
    case "code":
      return `<polyline points="8 6 3 12 8 18"/><polyline points="16 6 21 12 16 18"/>`;
    case "puzzle":
      return `<path d="M9 4h2a2 2 0 0 1 2 2v1h1a2 2 0 0 1 2 2v0a2 2 0 0 1-2 2h-1v1a2 2 0 0 1-2 2H9a2 2 0 0 1-2-2v-1H6a2 2 0 0 1-2-2v0a2 2 0 0 1 2-2h1V6a2 2 0 0 1 2-2z"/>`;
    case "terminal":
      return `<polyline points="4 7 8 11 4 15"/><line x1="12" y1="15" x2="20" y2="15"/>`;
    case "cog":
      return `<circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.93 4.93l2.12 2.12M16.95 16.95l2.12 2.12M4.93 19.07l2.12-2.12M16.95 7.05l2.12-2.12"/>`;
    default:
      return "";
  }
}

function isActive(to: string) {
  if (to === "/") return route.path === "/" || route.path === "/products";
  return route.path.startsWith(to);
}
</script>

<template>
  <aside class="sidebar">
    <div class="brand">
      <div class="brand-mark">ja</div>
      <div class="brand-text">
        <div class="brand-name">ja-netfilter</div>
        <div class="brand-sub">桌面版 · v{{ version }}</div>
      </div>
    </div>

    <nav>
      <router-link
        v-for="item in nav"
        :key="item.to"
        :to="item.to"
        class="nav-item"
        :class="{ active: isActive(item.to) }"
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" v-html="iconSvg(item.icon)" />
        <span>{{ item.label }}</span>
      </router-link>
    </nav>

    <div class="sidebar-footer">
      <div class="jar-status">
        <span class="dot" :class="jarOk ? 'ok' : 'error'" />
        <span class="jar-status-text">
          {{ jarOk ? "lib.jar 已就绪" : "lib.jar 缺失" }}
        </span>
      </div>
      <p class="footer-note">
        JetBrains IDE 的 Java agent 框架。<br />
        请仅在您拥有合法授权的软件上使用。
      </p>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  background: var(--bg-1);
  border-right: 1px solid var(--border-soft);
  display: flex;
  flex-direction: column;
  padding: 18px 12px;
  gap: 18px;
  overflow: hidden;
  height: 100vh;
}

.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 4px 8px 14px;
  border-bottom: 1px solid var(--border-soft);
}
.brand-mark {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  background: var(--accent);
  color: #fff;
  font-weight: 700;
  font-size: 14px;
  letter-spacing: -0.02em;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 2px 6px rgba(254, 90, 61, 0.35);
}
.brand-text {
  display: flex;
  flex-direction: column;
  line-height: 1.25;
}
.brand-name {
  font-weight: 600;
  font-size: 14px;
  color: var(--text-0);
}
.brand-sub {
  color: var(--text-2);
  font-size: 11px;
  letter-spacing: 0.02em;
}

nav {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1;
  overflow: auto;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: var(--radius);
  color: var(--text-1);
  font-size: 13px;
  font-weight: 500;
  transition: background var(--transition), color var(--transition);
}
.nav-item:hover {
  background: var(--bg-2);
  color: var(--text-0);
}
.nav-item.active {
  background: var(--accent-soft);
  color: var(--accent);
}
.nav-item.active svg {
  color: var(--accent);
}

.sidebar-footer {
  padding: 12px 8px 4px;
  border-top: 1px solid var(--border-soft);
}

.jar-status {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: var(--text-1);
  margin-bottom: 8px;
}
.jar-status-text {
  font-family: var(--font-mono);
  font-size: 11.5px;
}

.footer-note {
  margin: 0;
  color: var(--text-3);
  font-size: 11px;
  line-height: 1.4;
}
</style>
