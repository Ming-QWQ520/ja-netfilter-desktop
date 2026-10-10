<script setup lang="ts">
import { computed, onMounted, ref } from "vue";

import { api } from "@/api";
import { useT } from "@/i18n";
import { useProductsStore } from "@/stores/products";
import { useLicenseStore, DEFAULT_LICENSE_NAME, DEFAULT_EXPIRY_DATE } from "@/stores/license";
import { useSettingsStore } from "@/stores/settings";
import { useToast } from "@/stores/toast";
import { confirmDialog } from "@/stores/confirm";
import type { PluginJar, ProductInfo, VmoptionsSource } from "@/types";

const t = useT();
const store = useProductsStore();
const license = useLicenseStore();
const settings = useSettingsStore();
const toast = useToast();

const licenseNamePlaceholder = DEFAULT_LICENSE_NAME;
const licenseExpiryPlaceholder = DEFAULT_EXPIRY_DATE;

const plugins = ref<PluginJar[]>([]);
const busyAll = ref(false);
const busyId = ref<string | null>(null);

const licenseInput = ref(license.name);
const expiryInput = ref(license.expiry);
const saveLicense = () => {
  license.setName(licenseInput.value);
  license.setExpiry(expiryInput.value);
  licenseInput.value = license.name;
  expiryInput.value = license.expiry;
  toast.success(t("license_saved"));
};

const detected = computed(() => store.items.filter((p) => p.ide_found || p.vmoptions_source !== "missing"));
const installedCount = computed(() => store.items.filter((p) => p.javaagent_installed).length);
const detectedCount = computed(() => detected.value.length);
const agentOk = computed(() => settings.info?.jar_exists ?? false);

function sourceLabel(src: VmoptionsSource): string {
  return {
    user: t("source_user"),
    ide: t("source_ide"),
    template: t("source_template"),
    missing: t("source_missing"),
  }[src];
}

async function refreshAll() {
  await Promise.all([
    store.refreshAll(),
    settings.refresh(),
    loadPlugins(),
  ]);
}

async function loadPlugins() {
  try {
    plugins.value = await api.readPluginJars();
  } catch {
    /* 插件列表失败不打断主流程 */
  }
}

async function installOne(p: ProductInfo) {
  busyId.value = p.id;
  try {
    const r = await store.install(p.id, license.name, license.expiry);
    if (r?.success) toast.success(`${p.name}: ${t("install_done")}`);
    else toast.warn(`${p.name}: ${r?.message || t("load_failed")}`);
  } catch (e: any) {
    toast.error(`${p.name}: ${e}`);
  } finally {
    busyId.value = null;
  }
}

async function uninstallOne(p: ProductInfo) {
  busyId.value = p.id;
  try {
    await store.uninstall(p.id);
    toast.success(`${p.name}: ${t("uninstall_done")}`);
  } catch (e: any) {
    toast.error(`${p.name}: ${e}`);
  } finally {
    busyId.value = null;
  }
}

async function installAll() {
  const ok = await confirmDialog(t("confirm_install_all"));
  if (!ok) return;
  busyAll.value = true;
  try {
    const results = await store.installAll(license.name, license.expiry);
    const okCount = results.filter((r) => r.success).length;
    if (okCount > 0) toast.success(`${t("install_all_done")} (${okCount})`);
    else toast.warn(t("no_products"));
  } catch (e: any) {
    toast.error(`${t("install_failed")}: ${e}`);
  } finally {
    busyAll.value = false;
  }
}

async function uninstallAll() {
  const ok = await confirmDialog(t("confirm_uninstall_all"), true);
  if (!ok) return;
  busyAll.value = true;
  try {
    await store.uninstallAll();
    toast.success(t("uninstall_all_done"));
  } catch (e: any) {
    toast.error(String(e));
  } finally {
    busyAll.value = false;
  }
}

async function cleanEnv() {
  const ok = await confirmDialog(t("confirm_clean_env"), true);
  if (!ok) return;
  busyAll.value = true;
  try {
    await api.cleanupEnvVars();
    toast.success(t("env_cleaned"));
    await store.refreshAll();
  } catch (e: any) {
    toast.error(String(e));
  } finally {
    busyAll.value = false;
  }
}

function pluginDesc(name: string): string {
  const key = name.replace(".jar", "");
  const base = t("plugin_docs");
  return base;
}

onMounted(refreshAll);
</script>

<template>
  <section class="tab-panel">
    <!-- 工具区 -->
    <div class="hero md-card md-card--filled">
      <div class="hero-main">
        <div class="hero-stats">
          <span class="md-chip md-chip--success">
            {{ t("stat_installed") }} {{ installedCount }} / {{ store.items.length }}
          </span>
          <span class="md-chip md-chip--primary">
            {{ t("stat_detected") }} {{ detectedCount }}
          </span>
          <span class="md-chip" :class="agentOk ? 'md-chip--success' : 'md-chip--error'">
            {{ t("stat_agent_ready") }} {{ agentOk ? "✓" : "✕" }}
          </span>
        </div>
        <div class="md-field md-field--mono license-field">
          <span class="md-field__label">{{ t("license_name_label") }}</span>
          <div class="license-row">
            <input
              v-model="licenseInput"
              type="text"
              :placeholder="licenseNamePlaceholder"
              spellcheck="false"
              @keyup.enter="saveLicense"
            />
            <input
              v-model="expiryInput"
              type="text"
              class="expiry-input"
              :placeholder="licenseExpiryPlaceholder"
              spellcheck="false"
              @keyup.enter="saveLicense"
            />
            <button class="md-btn md-btn--tonal md-btn--sm" @click="saveLicense">
              {{ t("save") }}
            </button>
          </div>
          <p class="license-hint">{{ t("license_hint") }}</p>
        </div>
      </div>
      <div class="hero-actions">
        <button class="md-btn md-btn--text md-btn--sm" :disabled="store.loading" @click="refreshAll">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" class="md-btn__icon"><path d="M20 12a8 8 0 1 1-2.3-5.6"/><path d="M20 3v4h-4"/></svg>
          {{ t("refresh") }}
        </button>
        <button class="md-btn md-btn--outlined md-btn--sm" :disabled="busyAll" @click="cleanEnv">
          {{ t("clean_env") }}
        </button>
        <button class="md-btn md-btn--outlined md-btn--sm" :disabled="busyAll" @click="uninstallAll">
          {{ t("uninstall_all") }}
        </button>
        <button class="md-btn md-btn--filled md-btn--sm" :disabled="busyAll" @click="installAll">
          {{ busyAll ? "…" : t("install_all") }}
        </button>
      </div>
    </div>
    <div v-if="busyAll" class="md-progress"><div class="md-progress__bar" /></div>

    <!-- 产品卡片 -->
    <div v-if="store.loading && !store.items.length" class="grid-products">
      <div v-for="i in 6" :key="i" class="md-skeleton product-skeleton" />
    </div>
    <div v-else-if="!store.items.length" class="md-empty">
      <p>{{ t("no_products") }}</p>
    </div>
    <div v-else class="grid-products">
      <article
        v-for="p in store.items"
        :key="p.id"
        class="md-card md-card--elevated product-card"
        :class="{ 'is-installed': p.javaagent_installed, 'is-dim': !p.ide_found && !p.javaagent_installed }"
      >
        <header class="pc-head">
          <div class="pc-title">
            <span class="pc-dot" :class="p.javaagent_installed ? 'ok' : p.ide_found ? 'idle' : 'off'" />
            <span class="pc-name">{{ p.name }}</span>
          </div>
          <span class="md-chip" :class="p.javaagent_installed ? 'md-chip--success' : 'md-chip--outlined'">
            {{ p.javaagent_installed ? t("installed_badge") : t("not_installed_badge") }}
          </span>
        </header>

        <div class="pc-meta mono">
          <div class="pc-row">
            <span class="pc-k">id</span>
            <span class="pc-v">{{ p.id }}</span>
          </div>
          <div class="pc-row">
            <span class="pc-k">{{ t("vmoptions_label") }}</span>
            <span class="pc-v pc-path" :title="p.vmoptions_path ?? ''">
              {{ p.vmoptions_path ?? "—" }}
            </span>
          </div>
          <div class="pc-row">
            <span class="pc-k">source</span>
            <span class="md-chip md-chip--outlined md-chip--xs" :class="{ 'md-chip--error': p.vmoptions_source === 'missing' }">
              {{ sourceLabel(p.vmoptions_source) }}
            </span>
          </div>
        </div>

        <p v-if="!p.ide_found && !p.javaagent_installed" class="pc-hint">
          {{ t("ide_not_found") }}
        </p>
        <div v-else-if="p.javaagent_target" class="pc-agent mono" :title="p.javaagent_target">
          <code>{{ p.javaagent_target }}</code>
        </div>

        <footer class="pc-actions">
          <button
            v-if="p.javaagent_installed"
            class="md-btn md-btn--danger-outlined md-btn--sm"
            :disabled="busyId === p.id"
            @click="uninstallOne(p)"
          >
            {{ t("btn_uninstall") }}
          </button>
          <button
            class="md-btn md-btn--sm"
            :class="p.javaagent_installed ? 'md-btn--tonal' : 'md-btn--filled'"
            :disabled="busyId === p.id"
            @click="installOne(p)"
          >
            {{ busyId === p.id ? "…" : p.javaagent_installed ? t("btn_reinstall") : t("btn_install") }}
          </button>
        </footer>
      </article>
    </div>

    <!-- 插件 -->
    <div class="section-head">
      <h3>{{ t("plugins_title") }}</h3>
      <p>{{ t("plugins_desc") }}</p>
    </div>
    <div v-if="plugins.length" class="grid-plugins">
      <article v-for="p in plugins" :key="p.path" class="md-card md-card--outlined plugin-card">
        <header class="pl-head">
          <span class="pl-name mono">{{ p.name }}</span>
          <span class="md-chip md-chip--outlined md-chip--xs">{{ (p.size / 1024).toFixed(1) }} KB</span>
        </header>
        <p class="pl-desc">{{ pluginDesc(p.name) }}</p>
        <code class="pl-path mono selectable" :title="p.path">{{ p.path }}</code>
      </article>
    </div>
    <div v-else class="md-empty">
      <p>{{ t("no_plugins") }}</p>
    </div>
  </section>
</template>

<style scoped>
.tab-panel {
  height: 100%;
  overflow: auto;
  padding: 20px 24px 32px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

/* hero */
.hero {
  display: flex;
  align-items: stretch;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
}
.hero-main {
  display: flex;
  flex-direction: column;
  gap: 12px;
  flex: 1;
  min-width: 300px;
}
.hero-stats {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.license-field {
  max-width: 560px;
}
.license-row {
  display: flex;
  gap: 8px;
}
.license-row input {
  flex: 1;
}
.license-row .expiry-input {
  flex: 0 0 130px;
}
.license-hint {
  margin: 4px 0 0;
  font-size: 11.5px;
  color: var(--md-on-surface-variant);
  line-height: 1.5;
}
.hero-actions {
  display: flex;
  align-items: flex-end;
  gap: 8px;
  flex-wrap: wrap;
}

.grid-products {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 14px;
}
.product-skeleton {
  height: 190px;
}

.product-card {
  display: flex;
  flex-direction: column;
  gap: 10px;
  border-left: 3px solid transparent;
}
.product-card.is-installed {
  border-left-color: var(--md-success);
}
.product-card.is-dim {
  opacity: 0.72;
}

.pc-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.pc-title {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}
.pc-name {
  font-weight: 600;
  font-size: 15px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.pc-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}
.pc-dot.ok {
  background: var(--md-success);
}
.pc-dot.idle {
  background: var(--md-outline);
}
.pc-dot.off {
  background: var(--md-outline-variant);
}

.pc-meta {
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 12px;
}
.pc-row {
  display: grid;
  grid-template-columns: 84px 1fr;
  gap: 10px;
  align-items: baseline;
}
.pc-k {
  color: var(--md-on-surface-variant);
  font-size: 11px;
  letter-spacing: 0.04em;
}
.pc-v {
  color: var(--md-on-surface);
  word-break: break-all;
  line-height: 1.4;
}
.pc-path {
  font-size: 11.5px;
  color: var(--md-on-surface-variant);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.md-chip--xs {
  height: 22px;
  padding: 0 8px;
  font-size: 11px;
  width: fit-content;
}

.pc-hint {
  margin: 0;
  font-size: 12px;
  color: var(--md-warning);
}
.pc-agent {
  background: var(--md-surface-container);
  border-radius: var(--md-corner-s);
  padding: 8px 10px;
  font-size: 11px;
}
.pc-agent code {
  color: var(--md-success);
  word-break: break-all;
  white-space: pre-wrap;
}

.pc-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: auto;
}

.section-head {
  margin-top: 12px;
}
.section-head h3 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}
.section-head p {
  margin: 2px 0 0;
  font-size: 12.5px;
  color: var(--md-on-surface-variant);
}

.grid-plugins {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 12px;
}
.plugin-card {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.pl-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.pl-name {
  font-weight: 600;
  font-size: 14px;
  color: var(--md-primary);
}
.pl-desc {
  margin: 0;
  font-size: 12.5px;
  color: var(--md-on-surface-variant);
  line-height: 1.5;
}
.pl-path {
  font-size: 11px;
  color: var(--md-on-surface-variant);
  word-break: break-all;
  background: var(--md-surface-container);
  border-radius: var(--md-corner-xs);
  padding: 6px 8px;
}
</style>
