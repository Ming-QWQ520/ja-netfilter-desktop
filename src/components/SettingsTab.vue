<script setup lang="ts">
import { computed, onMounted, ref } from "vue";

import { useT } from "@/i18n";
import { useSettingsStore } from "@/stores/settings";
import {
  useLicenseStore,
  DEFAULT_LICENSE_NAME,
  DEFAULT_EXPIRY_DATE,
} from "@/stores/license";
import { useToast } from "@/stores/toast";
import { api } from "@/api";
import type { SyncSummary } from "@/types";

const t = useT();
const settings = useSettingsStore();
const license = useLicenseStore();
const toast = useToast();

const licenseInput = ref(license.name);
const expiryInput = ref(license.expiry);
const syncing = ref(false);
const syncResult = ref<SyncSummary | null>(null);

const info = computed(() => settings.info);

const osLabel = computed(() => {
  switch (info.value?.os) {
    case "windows":
      return "Windows";
    case "macos":
      return "macOS";
    case "linux":
      return "Linux";
    default:
      return "…";
  }
});

function saveLicense() {
  license.setName(licenseInput.value);
  license.setExpiry(expiryInput.value);
  licenseInput.value = license.name;
  expiryInput.value = license.expiry;
  toast.success(t("license_saved"));
}

async function syncAgent() {
  if (syncing.value) return;
  syncing.value = true;
  syncResult.value = null;
  try {
    const s = await api.syncAgentResources();
    syncResult.value = s;
    if (s.failed.length === 0) {
      toast.success(s.message);
    } else {
      toast.warn(s.message);
    }
  } catch (e: any) {
    toast.error(`${t("agent_sync_fail")}: ${String(e)}`);
  } finally {
    syncing.value = false;
  }
}

async function reveal(path: string) {
  try {
    await settings.reveal(path);
  } catch (e: any) {
    toast.error(String(e));
  }
}

onMounted(() => settings.refresh());
</script>

<template>
  <section class="tab-panel">
    <div class="settings-col">
      <!-- 授权 -->
      <div class="md-card">
        <h3>{{ t("license_title") }}</h3>
        <div class="license-row">
          <input
            v-model="licenseInput"
            type="text"
            class="md-input"
            :placeholder="DEFAULT_LICENSE_NAME"
            spellcheck="false"
            @keyup.enter="saveLicense"
          />
          <input
            v-model="expiryInput"
            type="text"
            class="md-input expiry-input"
            :placeholder="DEFAULT_EXPIRY_DATE"
            spellcheck="false"
            @keyup.enter="saveLicense"
          />
          <button class="md-btn md-btn--tonal md-btn--sm" @click="saveLicense">
            {{ t("save") }}
          </button>
        </div>
        <p class="hint">{{ t("license_hint") }}</p>
      </div>

      <!-- Agent 资源同步 -->
      <div class="md-card">
        <h3>{{ t("agent_sync_title") }}</h3>
        <div class="license-row">
          <button
            class="md-btn md-btn--tonal md-btn--sm"
            :disabled="syncing"
            @click="syncAgent"
          >
            {{ syncing ? t("agent_sync_running") : t("agent_sync_btn") }}
          </button>
          <span v-if="syncResult" class="hint sync-result">{{ syncResult.message }}</span>
        </div>
        <p class="hint">{{ t("agent_sync_desc") }}</p>
      </div>

      <!-- 路径 -->
      <div v-if="info" class="md-card">
        <h3>{{ t("paths") }}</h3>
        <div class="path-row">
          <span class="path-label">{{ t("path_agent_root") }}</span>
          <code class="path mono selectable" :title="info.agent_root">{{ info.agent_root }}</code>
          <span class="md-chip md-chip--xs" :class="info.agent_clean ? 'md-chip--success' : 'md-chip--warning'">
            {{ info.agent_clean ? "✓" : "⚠" }}
          </span>
          <button class="md-btn md-btn--text md-btn--sm" @click="reveal(info.agent_root)">
            {{ t("reveal") }}
          </button>
        </div>
        <div class="path-row">
          <span class="path-label">{{ t("agent_line_label") }}</span>
          <code class="path mono selectable" :title="info.jar_path">-javaagent:{{ info.jar_path }}=jetbrains</code>
          <span class="md-chip md-chip--xs" :class="info.jar_exists ? 'md-chip--success' : 'md-chip--error'">
            {{ info.jar_exists ? "✓" : "✕" }}
          </span>
          <button class="md-btn md-btn--text md-btn--sm" @click="reveal(info.jar_path)">
            {{ t("reveal") }}
          </button>
        </div>
      </div>

      <!-- 关于 -->
      <div class="md-card">
        <div class="about-row">
          <h3>{{ t("about") }}</h3>
          <span class="md-chip md-chip--outlined">v{{ info?.app_version ?? "…" }}</span>
          <span class="md-chip md-chip--outlined">{{ osLabel }}</span>
        </div>
        <p class="disclaimer">{{ t("disclaimer") }}</p>
      </div>
    </div>
  </section>
</template>

<style scoped>
.tab-panel {
  height: 100%;
  overflow: auto;
  padding: 14px 16px 24px;
}

.settings-col {
  max-width: 680px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.settings-col h3 {
  margin: 0 0 10px;
  font-size: 14px;
  font-weight: 600;
}

.license-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.license-row .md-input {
  flex: 1;
  min-width: 0;
}
.license-row .expiry-input {
  flex: 0 0 120px;
}

.md-input {
  height: 32px;
  padding: 0 10px;
  border: none;
  border-radius: var(--md-corner-xs);
  background: var(--md-surface-container-highest);
  color: var(--md-on-surface);
  font: 400 12.5px/1.4 var(--md-mono);
  outline: none;
  box-shadow: inset 0 0 0 1px var(--md-outline-variant);
  transition: box-shadow var(--md-dur-short);
  width: 100%;
}
.md-input:focus {
  box-shadow: inset 0 0 0 2px var(--md-primary);
}

.hint {
  margin: 8px 0 0;
  font-size: 11.5px;
  color: var(--md-on-surface-variant);
  line-height: 1.55;
  word-break: break-all;
}
.sync-result {
  margin: 0;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.path-row {
  display: grid;
  grid-template-columns: 96px 1fr auto auto;
  gap: 8px;
  align-items: center;
  padding: 5px 0;
  min-width: 0;
}
.path-label {
  font-size: 12px;
  color: var(--md-on-surface-variant);
}
.path {
  min-width: 0;
  font-size: 11.5px;
  background: var(--md-surface-container);
  border-radius: var(--md-corner-xs);
  padding: 5px 8px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.md-chip--xs {
  height: 20px;
  padding: 0 7px;
  font-size: 11px;
  flex-shrink: 0;
}

.about-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.about-row h3 {
  margin: 0;
  flex: 1;
}
.disclaimer {
  margin: 0;
  font-size: 11.5px;
  line-height: 1.6;
  padding: 10px 12px;
  border-radius: var(--md-corner-m);
  background: var(--md-warning-container);
  color: var(--md-on-warning-container);
}

@media (max-width: 720px) {
  .path-row {
    grid-template-columns: 1fr auto;
  }
  .path-label {
    grid-column: 1 / -1;
  }
}
</style>
