<script setup lang="ts">
import { computed, onMounted, ref } from "vue";

import { api } from "@/api";
import { useT } from "@/i18n";
import { useProductsStore } from "@/stores/products";
import { useLicenseStore } from "@/stores/license";
import { useSettingsStore } from "@/stores/settings";
import { useToast } from "@/stores/toast";
import { confirmDialog } from "@/stores/confirm";
import type { PluginJar, ProductInfo } from "@/types";

const t = useT();
const store = useProductsStore();
const license = useLicenseStore();
const settings = useSettingsStore();
const toast = useToast();

const plugins = ref<PluginJar[]>([]);
const busyAll = ref(false);
const busyId = ref<string | null>(null);

const installedCount = computed(() => store.items.filter((p) => p.javaagent_installed).length);
const detectedCount = computed(() => store.items.filter((p) => p.ide_found).length);
const agentOk = computed(() => settings.info?.jar_exists ?? false);

async function refreshAll() {
  await Promise.all([store.refreshAll(), settings.refresh(), loadPlugins()]);
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

onMounted(refreshAll);
</script>

<template>
  <section class="tab-panel">
    <!-- 工具行：状态 + 批量操作 -->
    <div class="toolbar">
      <div class="stats">
        <span class="stat-pill">
          <span class="stat-dot ok" />
          {{ t("stat_installed") }}
          <b class="mono">{{ installedCount }} / {{ store.items.length }}</b>
        </span>
        <span class="stat-pill">
          <span class="stat-dot info" />
          {{ t("stat_detected") }}
          <b class="mono">{{ detectedCount }}</b>
        </span>
        <span class="stat-pill" :class="agentOk ? 'is-ok' : 'is-bad'">
          <span class="stat-dot" :class="agentOk ? 'ok' : 'bad'" />
          {{ t("stat_agent_ready") }}
          <b class="mono">{{ agentOk ? "✓" : "✕" }}</b>
        </span>
      </div>
      <div class="actions">
        <button class="md-icon-btn" :title="t('refresh')" :disabled="store.loading" @click="refreshAll">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M20 12a8 8 0 1 1-2.3-5.6"/><path d="M20 3v4h-4"/></svg>
        </button>
        <button class="md-btn md-btn--text md-btn--sm" :disabled="busyAll" @click="cleanEnv">
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

    <!-- 产品列表 -->
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
        class="md-card product-card"
        :class="{ 'is-installed': p.javaagent_installed, 'is-dim': !p.ide_found && !p.javaagent_installed }"
      >
        <header class="pc-head">
          <span class="pc-dot" :class="p.javaagent_installed ? 'ok' : p.ide_found ? 'idle' : 'off'" />
          <span class="pc-name" :title="p.id">{{ p.name }}</span>
          <span class="md-chip md-chip--xs" :class="p.javaagent_installed ? 'md-chip--success' : 'md-chip--outlined'">
            {{ p.javaagent_installed ? t("installed_badge") : t("not_installed_badge") }}
          </span>
        </header>

        <p v-if="!p.ide_found && !p.javaagent_installed" class="pc-hint">
          {{ t("ide_not_found") }}
        </p>
        <template v-else>
          <div v-if="p.javaagent_target" class="pc-agent mono" :title="p.javaagent_target">
            {{ p.javaagent_target }}
          </div>
          <div v-else class="pc-vmopts mono" :title="p.vmoptions_path ?? ''">
            {{ p.vmoptions_path ?? "—" }}
          </div>
        </template>

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
      <p>{{ t("plugin_docs") }}</p>
    </div>
    <div v-if="plugins.length" class="plugin-list">
      <div v-for="p in plugins" :key="p.path" class="plugin-row" :title="p.path">
        <span class="pl-name mono">{{ p.name }}</span>
        <span class="pl-size mono">{{ (p.size / 1024).toFixed(1) }} KB</span>
        <span class="pl-path mono">{{ p.path }}</span>
      </div>
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
  padding: 16px 20px 28px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

/* 工具行 */
.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}
.stats {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.stat-pill {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  height: 28px;
  padding: 0 12px;
  border-radius: var(--md-corner-full);
  background: var(--md-surface-container-lowest);
  box-shadow: inset 0 0 0 1px var(--md-outline-variant);
  font: 500 12px/1 var(--md-font);
  color: var(--md-on-surface-variant);
}
.stat-pill b {
  font-weight: 600;
  color: var(--md-on-surface);
}
.stat-pill.is-ok b {
  color: var(--md-success);
}
.stat-pill.is-bad b {
  color: var(--md-error);
}
.stat-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
}
.stat-dot.ok {
  background: var(--md-success);
}
.stat-dot.info {
  background: var(--md-tertiary);
}
.stat-dot.bad {
  background: var(--md-error);
}
.actions {
  display: flex;
  align-items: center;
  gap: 6px;
}

.grid-products {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 12px;
}
.product-skeleton {
  height: 124px;
}

.product-card {
  display: flex;
  flex-direction: column;
  gap: 9px;
  padding: 14px;
}
.product-card:hover {
  box-shadow: inset 0 0 0 1px var(--md-outline-variant), var(--md-shadow-1);
}
.product-card.is-installed:hover {
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--md-success) 35%, transparent), var(--md-shadow-1);
}
.product-card.is-dim {
  opacity: 0.72;
}

.pc-head {
  display: flex;
  align-items: center;
  gap: 8px;
}
.pc-name {
  font-weight: 600;
  font-size: 13.5px;
  flex: 1;
  min-width: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.pc-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.pc-dot.ok {
  background: var(--md-success);
  box-shadow: 0 0 0 3px var(--md-success-container);
}
.pc-dot.idle {
  background: var(--md-outline);
}
.pc-dot.off {
  background: var(--md-outline-variant);
}

.pc-agent,
.pc-vmopts {
  font-size: 11.5px;
  line-height: 1.45;
  padding: 5px 9px;
  border-radius: var(--md-corner-xs);
  background: var(--md-surface-container);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.pc-agent {
  color: var(--md-success);
  background: var(--md-success-container);
}
.pc-vmopts {
  color: var(--md-on-surface-variant);
}

.pc-hint {
  margin: 0;
  font-size: 12px;
  color: var(--md-warning);
}

.pc-actions {
  display: flex;
  justify-content: flex-end;
  gap: 6px;
  margin-top: auto;
}

.section-head {
  margin-top: 8px;
}
.section-head h3 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}
.section-head p {
  margin: 3px 0 0;
  font-size: 12px;
  color: var(--md-on-surface-variant);
  line-height: 1.5;
}

/* 插件紧凑列表 */
.plugin-list {
  display: flex;
  flex-direction: column;
  background: var(--md-surface-container-lowest);
  box-shadow: inset 0 0 0 1px var(--md-outline-variant);
  border-radius: var(--md-corner-l);
  overflow: hidden;
}
.plugin-row {
  display: grid;
  grid-template-columns: 120px 72px 1fr;
  gap: 12px;
  align-items: center;
  padding: 8px 14px;
  font-size: 12px;
  transition: background-color var(--md-dur-short);
}
.plugin-row + .plugin-row {
  border-top: 1px solid var(--md-outline-variant);
}
.plugin-row:hover {
  background: var(--md-surface-container-low);
}
.pl-name {
  font-weight: 600;
  color: var(--md-primary);
}
.pl-size {
  color: var(--md-on-surface-variant);
  text-align: right;
}
.pl-path {
  color: var(--md-on-surface-variant);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

@media (max-width: 720px) {
  .plugin-row {
    grid-template-columns: 100px 64px 1fr;
    gap: 8px;
  }
}
</style>
