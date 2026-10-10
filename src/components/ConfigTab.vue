<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";

import { api } from "@/api";
import { useT } from "@/i18n";
import { ui } from "@/stores/ui";
import { useProductsStore } from "@/stores/products";
import { useToast } from "@/stores/toast";
import { confirmDialog } from "@/stores/confirm";
import type { ConfigFile } from "@/types";

type Mode = "configs" | "vmoptions";

const t = useT();
const toast = useToast();
const productsStore = useProductsStore();

const mode = ref<Mode>("configs");

/* ---------------- 共用编辑器状态 ---------------- */
const target = ref<string | null>(null); // config: relative_path；vmoptions: path_or_id
const content = ref("");
const original = ref("");
const loading = ref(false);
const saving = ref(false);
const stripLabel = ref(false); // vmoptions 模式下目标是否为真实 IDE 文件
const vmTitle = ref("");

const dirty = computed(() => content.value !== original.value);

/* ---------------- 插件配置 ---------------- */
const configs = ref<ConfigFile[]>([]);

const configDesc: Record<string, { zh: string; en: string }> = {
  "dns.conf": {
    zh: "拦截或改写 IDE 内部的域名解析请求。",
    en: "Intercept or rewrite in-IDE DNS resolution.",
  },
  "power.conf": {
    zh: "agent 内部使用的 RSA 密钥 / 指数变换规则。",
    en: "RSA key / exponent transform rules used by the agent.",
  },
  "url.conf": {
    zh: "拦截匹配的对外 URL 请求并直接返回本地响应。",
    en: "Intercept matching outbound URLs and answer locally.",
  },
  "env.conf": {
    zh: "env 插件规则（环境/系统属性）。",
    en: "env plugin rules (environment / system properties).",
  },
  "native.conf": {
    zh: "native 插件规则。",
    en: "native plugin rules.",
  },
};

function configDescFor(name: string): string {
  const entry = configDesc[name];
  if (!entry) return "";
  return uiLocale.value === "en" ? entry.en : entry.zh;
}

const uiLocale = computed(() => ui().locale);

async function loadConfigs() {
  loading.value = true;
  try {
    configs.value = await api.listConfigs();
    if (!target.value || !configs.value.some((c) => c.relative_path === target.value)) {
      target.value = configs.value[0]?.relative_path ?? null;
    }
  } catch (e: any) {
    toast.error(`${t("load_failed")}: ${e}`);
  } finally {
    loading.value = false;
  }
}

/* ---------------- vmoptions ---------------- */
const vmProducts = computed(() =>
  [...productsStore.items].sort((a, b) => {
    if (a.ide_found !== b.ide_found) return a.ide_found ? -1 : 1;
    return a.name.localeCompare(b.name);
  }),
);

async function loadVmFor(id: string) {
  loading.value = true;
  try {
    const info = productsStore.items.find((p) => p.id === id);
    const path = info?.vmoptions_path;
    stripLabel.value = !!path;
    vmTitle.value = path ?? `${id}.vmoptions (template)`;
    const text = await api.readVmoptions(path ?? id);
    content.value = text;
    original.value = text;
  } catch (e: any) {
    toast.error(String(e));
    content.value = "";
    original.value = "";
  } finally {
    loading.value = false;
  }
}

async function switchMode(m: Mode) {
  mode.value = m;
  target.value = null;
  content.value = "";
  original.value = "";
  if (m === "configs") await loadConfigs();
  else {
    if (!productsStore.items.length) await productsStore.refreshAll();
    target.value = vmProducts.value[0]?.id ?? null;
  }
}

async function save() {
  if (!target.value) return;
  saving.value = true;
  try {
    if (mode.value === "configs") {
      await api.writeConfig(target.value, content.value);
    } else {
      const info = productsStore.items.find((p) => p.id === target.value);
      const path = info?.vmoptions_path ?? target.value;
      await api.writeVmoptions(path, content.value);
    }
    original.value = content.value;
    toast.success(t("saved_ok"));
    if (mode.value === "configs") loadConfigs();
    else productsStore.refreshAll();
  } catch (e: any) {
    toast.error(String(e));
  } finally {
    saving.value = false;
  }
}

function revert() {
  content.value = original.value;
}

async function stripAgents() {
  const tgt = target.value;
  if (!tgt || mode.value !== "vmoptions") return;
  const ok = await confirmDialog(t("confirm_strip_vmoptions"), true);
  if (!ok) return;
  try {
    const info = productsStore.items.find((p) => p.id === tgt);
    const path = info?.vmoptions_path ?? tgt;
    await api.stripVmoptions(path);
    await loadVmFor(tgt);
    toast.success(t("agents_stripped"));
  } catch (e: any) {
    toast.error(String(e));
  }
}

async function loadContent() {
  const tgt = target.value;
  if (!tgt) return;
  loading.value = true;
  try {
    const text =
      mode.value === "configs"
        ? await api.readConfig(tgt)
        : await (async () => {
            const info = productsStore.items.find((p) => p.id === tgt);
            const path = info?.vmoptions_path;
            stripLabel.value = !!path;
            vmTitle.value = path ?? `${tgt}.vmoptions (template)`;
            return api.readVmoptions(path ?? tgt);
          })();
    content.value = text;
    original.value = text;
  } catch (e: any) {
    toast.error(String(e));
    content.value = "";
    original.value = "";
  } finally {
    loading.value = false;
  }
}

watch(target, () => loadContent());

onMounted(async () => {
  await loadConfigs();
});
</script>

<template>
  <section class="tab-panel">
    <div class="config-toolbar">
      <div class="md-segmented">
        <button :class="{ 'is-active': mode === 'configs' }" @click="switchMode('configs')">
          {{ t("config_files") }}
        </button>
        <button :class="{ 'is-active': mode === 'vmoptions' }" @click="switchMode('vmoptions')">
          {{ t("vmoptions_editor") }}
        </button>
      </div>
    </div>

    <div class="config-layout">
      <!-- 左侧列表 -->
      <aside class="side-list">
        <template v-if="mode === 'configs'">
          <button
            v-for="c in configs"
            :key="c.relative_path"
            class="md-list-item side-item"
            :class="{ 'is-active': c.relative_path === target }"
            @click="target = c.relative_path"
          >
            <span class="side-name mono">{{ c.name }}</span>
            <span class="side-sub">{{ (c.size / 1024).toFixed(2) }} KB</span>
          </button>
          <p v-if="configs.length" class="side-desc">{{ configDescFor(configs.find((c) => c.relative_path === target)?.name ?? "") }}</p>
        </template>
        <template v-else>
          <button
            v-for="p in vmProducts"
            :key="p.id"
            class="md-list-item side-item"
            :class="{ 'is-active': p.id === target }"
            @click="target = p.id"
          >
            <span class="side-dot" :class="p.javaagent_installed ? 'ok' : p.ide_found ? 'idle' : 'off'" />
            <span class="side-name">{{ p.name }}</span>
            <span class="side-sub mono">{{ p.id }}</span>
          </button>
        </template>
      </aside>

      <!-- 编辑区 -->
      <div class="editor-col">
        <div class="editor-head">
          <div class="editor-chips">
            <span v-if="target" class="md-chip md-chip--primary mono">
              {{ mode === "configs" ? target : vmTitle }}
            </span>
            <span v-if="dirty" class="md-chip md-chip--warning">{{ t("dirty") }}</span>
            <span v-else-if="target" class="md-chip md-chip--success">{{ t("synced") }}</span>
          </div>
          <div class="editor-actions">
            <template v-if="mode === 'vmoptions'">
              <button class="md-btn md-btn--text md-btn--sm" :disabled="!target" @click="stripAgents">
                {{ t("strip_agents") }}
              </button>
            </template>
            <button class="md-btn md-btn--text md-btn--sm" :disabled="!dirty" @click="revert">
              {{ t("revert") }}
            </button>
            <button class="md-btn md-btn--filled md-btn--sm" :disabled="!dirty || saving" @click="save">
              {{ saving ? t("saving") : t("save") }}
            </button>
          </div>
        </div>
        <textarea
          v-model="content"
          class="md-editor"
          spellcheck="false"
          :placeholder="t('no_file_selected')"
        />
      </div>
    </div>
  </section>
</template>

<style scoped>
.tab-panel {
  height: 100%;
  overflow: hidden;
  padding: 14px 16px 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.config-toolbar {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 16px;
  flex-wrap: wrap;
}

.config-layout {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: 230px 1fr;
  gap: 10px;
}

.side-list {
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 4px;
  background: var(--md-surface-container);
  border-radius: var(--md-corner-l);
}
.side-item {
  flex-shrink: 0;
}
.side-name {
  font-weight: 500;
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.side-sub {
  font-size: 11px;
  color: var(--md-on-surface-variant);
  flex-shrink: 0;
}
.side-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.side-dot.ok {
  background: var(--md-success);
}
.side-dot.idle {
  background: var(--md-outline);
}
.side-dot.off {
  background: var(--md-outline-variant);
}
.side-desc {
  margin: 8px 12px 12px;
  font-size: 11.5px;
  color: var(--md-on-surface-variant);
  line-height: 1.5;
}

.editor-col {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-height: 0;
}
.editor-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}
.editor-chips {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}
.editor-chips .md-chip {
  max-width: 480px;
  overflow: hidden;
  text-overflow: ellipsis;
}
.editor-actions {
  display: flex;
  gap: 6px;
}
</style>
