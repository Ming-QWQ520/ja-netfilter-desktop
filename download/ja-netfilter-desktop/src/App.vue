<script setup lang="ts">
import { ref } from "vue";

import { ui, type TabId } from "@/stores/ui";
import { useT } from "@/i18n";
import { api } from "@/api";

import OverviewTab from "@/components/OverviewTab.vue";
import ConfigTab from "@/components/ConfigTab.vue";
import LogsTab from "@/components/LogsTab.vue";
import SettingsTab from "@/components/SettingsTab.vue";
import SnackbarHost from "@/components/SnackbarHost.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";

const t = useT();
const uiStore = ui();

const tabs: { id: TabId; labelKey: string; icon: string }[] = [
  { id: "overview", labelKey: "tab_overview", icon: "grid" },
  { id: "config", labelKey: "tab_config", icon: "sliders" },
  { id: "logs", labelKey: "tab_logs", icon: "terminal" },
  { id: "settings", labelKey: "tab_settings", icon: "cog" },
];

function iconSvg(name: string): string {
  switch (name) {
    case "grid":
      return `<rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/>`;
    case "sliders":
      return `<line x1="4" y1="7" x2="20" y2="7"/><line x1="4" y1="12" x2="20" y2="12"/><line x1="4" y1="17" x2="20" y2="17"/><circle cx="9" cy="7" r="2" class="knob"/><circle cx="15" cy="12" r="2" class="knob"/><circle cx="7" cy="17" r="2" class="knob"/>`;
    case "terminal":
      return `<polyline points="4 6 9 11 4 16"/><line x1="12" y1="16" x2="20" y2="16"/>`;
    case "cog":
      return `<circle cx="12" cy="12" r="3.2"/><path d="M12 2.5v3M12 18.5v3M2.5 12h3M18.5 12h3M5.2 5.2l2.1 2.1M16.7 16.7l2.1 2.1M5.2 18.8l2.1-2.1M16.7 7.3l2.1-2.1"/>`;
    default:
      return "";
  }
}

const version = ref("");

// 延迟获取版本号（避免阻塞首帧）
api.appVersion().then((v) => {
  version.value = v;
}).catch(() => {});

const sunIcon = `<circle cx="12" cy="12" r="4.2"/><path d="M12 2.5v2.4M12 19.1v2.4M2.5 12h2.4M19.1 12h2.4M5 5l1.7 1.7M17.3 17.3L19 19M5 19l1.7-1.7M17.3 6.7L19 5"/>`;
const moonIcon = `<path d="M20.5 14.5A8.5 8.5 0 1 1 9.5 3.5a7 7 0 0 0 11 11z"/>`;
</script>

<template>
  <header class="top-bar">
    <div class="brand">
      <div class="brand-mark mono">ja</div>
      <div class="brand-text">
        <div class="brand-name">{{ t("app_name") }}</div>
        <div class="brand-sub mono">v{{ version || "…" }}</div>
      </div>
    </div>

    <nav class="md-tabs" aria-label="primary">
      <button
        v-for="item in tabs"
        :key="item.id"
        class="md-tab"
        :class="{ 'is-active': uiStore.tab === item.id }"
        @click="uiStore.tab = item.id"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" v-html="iconSvg(item.icon)" />
        <span>{{ t(item.labelKey) }}</span>
      </button>
    </nav>

    <div class="bar-actions">
      <button class="md-segmented" :aria-label="t('language_label')">
        <button :class="{ 'is-active': uiStore.locale === 'zh' }" @click="uiStore.setLocale('zh')">中</button>
        <button :class="{ 'is-active': uiStore.locale === 'en' }" @click="uiStore.setLocale('en')">EN</button>
      </button>
      <button class="md-icon-btn" :title="t('theme_label')" @click="uiStore.toggleTheme()">
        <svg v-if="uiStore.isDark" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" v-html="sunIcon" />
        <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" v-html="moonIcon" />
      </button>
    </div>
  </header>

  <main class="content">
    <OverviewTab v-show="uiStore.tab === 'overview'" />
    <ConfigTab v-show="uiStore.tab === 'config'" />
    <LogsTab v-show="uiStore.tab === 'logs'" />
    <SettingsTab v-show="uiStore.tab === 'settings'" />
  </main>

  <SnackbarHost />
  <ConfirmDialog />
</template>

<style scoped>
.top-bar {
  display: flex;
  align-items: center;
  gap: 24px;
  height: 72px;
  padding: 0 20px;
  background: var(--md-surface-container);
  flex-shrink: 0;
}

.brand {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}
.brand-mark {
  width: 40px;
  height: 40px;
  border-radius: var(--md-corner-m);
  background: var(--md-primary);
  color: var(--md-on-primary);
  font-weight: 700;
  font-size: 15px;
  display: grid;
  place-content: center;
  box-shadow: var(--md-shadow-1);
  flex-shrink: 0;
}
.brand-text {
  line-height: 1.2;
  min-width: 0;
}
.brand-name {
  font-weight: 600;
  font-size: 15px;
  white-space: nowrap;
}
.brand-sub {
  font-size: 11px;
  color: var(--md-on-surface-variant);
}

.md-tabs {
  flex: 1;
  justify-content: center;
}

.bar-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.content {
  flex: 1;
  overflow: hidden;
  position: relative;
}
</style>
