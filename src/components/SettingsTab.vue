<script setup lang="ts">
import { computed, onMounted, ref } from "vue";

import { useT } from "@/i18n";
import { ui, type ThemeMode } from "@/stores/ui";
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
const uiStore = ui();
const settings = useSettingsStore();
const license = useLicenseStore();
const toast = useToast();

const licenseInput = ref(license.name);
const expiryInput = ref(license.expiry);
const syncing = ref(false);
const syncResult = ref<SyncSummary | null>(null);

const info = computed(() => settings.info);

const themes: { id: ThemeMode; labelKey: string }[] = [
  { id: "system", labelKey: "theme_system" },
  { id: "light", labelKey: "theme_light" },
  { id: "dark", labelKey: "theme_dark" },
];

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
      toast.success(s.message || t("agent_sync_done").replace("{msg}", ""));
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
    <div class="settings-grid">
      <!-- 外观 -->
      <div class="md-card md-card--elevated">
        <h3>{{ t("appearance") }}</h3>
        <div class="row">
          <div class="row-label">
            <span class="row-title">{{ t("theme_label") }}</span>
          </div>
          <div class="row-value theme-options">
            <label v-for="th in themes" :key="th.id" class="md-radio">
              <input
                type="radio"
                name="theme"
                :value="th.id"
                :checked="uiStore.theme === th.id"
                @change="uiStore.setTheme(th.id)"
              />
              <span>{{ t(th.labelKey) }}</span>
            </label>
          </div>
        </div>
        <div class="row">
          <div class="row-label">
            <span class="row-title">{{ t("language_label") }}</span>
          </div>
          <div class="row-value">
            <div class="md-segmented">
              <button
                :class="{ 'is-active': uiStore.locale === 'zh' }"
                @click="uiStore.setLocale('zh')"
              >
                中文
              </button>
              <button
                :class="{ 'is-active': uiStore.locale === 'en' }"
                @click="uiStore.setLocale('en')"
              >
                English
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- 行为 -->
      <div class="md-card md-card--elevated">
        <h3>{{ t("behavior") }}</h3>
        <div class="row">
          <div class="row-label">
            <span class="row-title">{{ t("license_name_label") }}</span>
            <p class="row-hint">{{ t("license_hint") }}</p>
          </div>
          <div class="row-value license-editor">
            <div class="md-field md-field--mono">
              <input
                v-model="licenseInput"
                type="text"
                :placeholder="DEFAULT_LICENSE_NAME"
                spellcheck="false"
                @keyup.enter="saveLicense"
              />
            </div>
            <div class="md-field md-field--mono">
              <input
                v-model="expiryInput"
                type="text"
                class="expiry-input"
                :placeholder="DEFAULT_EXPIRY_DATE"
                spellcheck="false"
                @keyup.enter="saveLicense"
              />
            </div>
            <button class="md-btn md-btn--tonal md-btn--sm" @click="saveLicense">
              {{ t("save") }}
            </button>
          </div>
        </div>
      </div>

      <!-- Agent 资源同步 -->
      <div class="md-card md-card--elevated">
        <h3>{{ t("agent_sync_title") }}</h3>
        <div class="row">
          <div class="row-label">
            <span class="row-title">ckey.run</span>
            <p class="row-hint">{{ t("agent_sync_desc") }}</p>
          </div>
          <div class="row-value">
            <button
              class="md-btn md-btn--tonal md-btn--sm"
              :disabled="syncing"
              @click="syncAgent"
            >
              {{ syncing ? t("agent_sync_running") : t("agent_sync_btn") }}
            </button>
            <p v-if="syncResult" class="row-hint sync-result">{{ syncResult.message }}</p>
          </div>
        </div>
      </div>

      <!-- 路径 -->
      <div v-if="info" class="md-card md-card--elevated">
        <h3>{{ t("paths") }}</h3>

        <div class="path-row">
          <div class="row-label">
            <span class="row-title">{{ t("path_agent_root") }}</span>
            <p class="row-hint">{{ t("path_agent_root_desc") }}</p>
          </div>
          <div class="row-value">
            <code class="path mono selectable">{{ info.agent_root }}</code>
            <div class="row-actions">
              <span class="md-chip" :class="info.agent_clean ? 'md-chip--success' : 'md-chip--warning'">
                {{ info.agent_clean ? "space-free ✓" : "contains spaces ⚠" }}
              </span>
              <button class="md-btn md-btn--text md-btn--sm" @click="reveal(info.agent_root)">
                {{ t("reveal") }}
              </button>
            </div>
          </div>
        </div>

        <hr class="md-divider" />

        <div class="path-row">
          <div class="row-label">
            <span class="row-title">lib.jar</span>
            <p class="row-hint">-javaagent:{{ info.jar_path }}=jetbrains</p>
          </div>
          <div class="row-value">
            <code class="path mono selectable">{{ info.jar_path }}</code>
            <div class="row-actions">
              <span class="md-chip" :class="info.jar_exists ? 'md-chip--success' : 'md-chip--error'">
                {{ info.jar_exists ? "✓" : "✕" }}
              </span>
              <button class="md-btn md-btn--text md-btn--sm" @click="reveal(info.jar_path)">
                {{ t("reveal") }}
              </button>
            </div>
          </div>
        </div>

        <hr class="md-divider" />

        <div class="path-row">
          <div class="row-label">
            <span class="row-title">{{ t("path_config") }}</span>
          </div>
          <div class="row-value">
            <code class="path mono selectable">{{ info.config_dir }}</code>
            <div class="row-actions">
              <button class="md-btn md-btn--text md-btn--sm" @click="reveal(info.config_dir)">
                {{ t("reveal") }}
              </button>
            </div>
          </div>
        </div>

        <hr class="md-divider" />

        <div class="path-row">
          <div class="row-label">
            <span class="row-title">{{ t("path_workdir") }}</span>
          </div>
          <div class="row-value">
            <code class="path mono selectable">{{ info.workdir }}</code>
            <div class="row-actions">
              <button class="md-btn md-btn--text md-btn--sm" @click="reveal(info.workdir)">
                {{ t("reveal") }}
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- 关于 -->
      <div class="md-card md-card--elevated">
        <h3>{{ t("about") }}</h3>
        <div class="about-row">
          <span class="md-chip md-chip--outlined">{{ t("version") }}: {{ info?.app_version ?? "…" }}</span>
          <span class="md-chip md-chip--outlined">{{ t("platform") }}: {{ osLabel }}</span>
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
  padding: 20px 24px 32px;
}

.settings-grid {
  max-width: 860px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.settings-grid h3 {
  margin: 0 0 14px;
  font-size: 16px;
  font-weight: 600;
}

.row {
  display: grid;
  grid-template-columns: 300px 1fr;
  gap: 16px;
  align-items: start;
  padding: 10px 0;
}
.row-label {
  min-width: 0;
}
.row-title {
  font-weight: 500;
  font-size: 14px;
}
.row-hint {
  margin: 4px 0 0;
  font-size: 11.5px;
  color: var(--md-on-surface-variant);
  word-break: break-all;
  line-height: 1.5;
}
.row-value {
  display: flex;
  flex-direction: column;
  gap: 8px;
  align-items: flex-start;
}
.theme-options {
  flex-direction: row;
  gap: 18px;
  flex-wrap: wrap;
}
.license-editor {
  flex-direction: row;
  align-items: center;
  gap: 8px;
  width: 100%;
}
.license-editor .md-field {
  flex: 1;
}
.sync-result {
  color: var(--md-on-surface-variant);
}

.path-row {
  display: grid;
  grid-template-columns: 300px 1fr;
  gap: 16px;
  padding: 12px 0;
  align-items: start;
}
.path {
  display: block;
  font-size: 12px;
  background: var(--md-surface-container);
  border-radius: var(--md-corner-s);
  padding: 8px 10px;
  word-break: break-all;
  line-height: 1.5;
}
.row-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.about-row {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
}
.disclaimer {
  margin: 0;
  font-size: 12.5px;
  color: var(--md-on-surface-variant);
  line-height: 1.65;
  padding: 12px 14px;
  border-radius: var(--md-corner-m);
  background: var(--md-warning-container);
  color: var(--md-on-warning-container);
}

@media (max-width: 760px) {
  .row,
  .path-row {
    grid-template-columns: 1fr;
  }
}
</style>
