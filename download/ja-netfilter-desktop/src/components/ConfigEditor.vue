<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";

import { api } from "@/api";
import { useToast } from "@/stores/toast";
import type { ConfigFile } from "@/types";

const toast = useToast();
const configs = ref<ConfigFile[]>([]);
const activePath = ref<string | null>(null);
const content = ref("");
const originalContent = ref("");
const loading = ref(false);
const saving = ref(false);

const dirty = computed(() => content.value !== originalContent.value);

const activeConfig = computed(() =>
  configs.value.find((c) => c.relative_path === activePath.value) ?? null
);

async function loadConfigs() {
  loading.value = true;
  try {
    configs.value = await api.listConfigs();
    if (!activePath.value && configs.value.length) {
      activePath.value = configs.value[0].relative_path;
    }
  } catch (e: any) {
    toast.error(String(e));
  } finally {
    loading.value = false;
  }
}

async function loadContent(path: string) {
  loading.value = true;
  try {
    const text = await api.readConfig(path);
    content.value = text;
    originalContent.value = text;
  } catch (e: any) {
    toast.error(String(e));
    content.value = "";
    originalContent.value = "";
  } finally {
    loading.value = false;
  }
}

async function save() {
  if (!activePath.value) return;
  saving.value = true;
  try {
    await api.writeConfig(activePath.value, content.value);
    originalContent.value = content.value;
    toast.success(`Saved ${activePath.value}`);
  } catch (e: any) {
    toast.error(String(e));
  } finally {
    saving.value = false;
  }
}

function revert() {
  content.value = originalContent.value;
}

watch(activePath, async (p) => {
  if (p) await loadContent(p);
});

onMounted(loadConfigs);

function descriptionFor(name: string): string {
  switch (name) {
    case "dns.conf":
      return "Plugin: dns — block or rewrite hostnames resolved by the IDE.";
    case "power.conf":
      return "Plugin: power — RSA key/value transformation rules used by the agent.";
    case "url.conf":
      return "Plugin: url — intercept matching outbound URLs and respond locally.";
    default:
      return "";
  }
}
</script>

<template>
  <section class="content-area">
    <header class="content-header">
      <div>
        <h2>Plugin Configs</h2>
        <p class="subtitle">Edit the dns / power / url plugin rules used by ja-netfilter.</p>
      </div>
      <div class="toolbar">
        <button class="ghost" :disabled="loading" @click="loadConfigs">Reload</button>
        <button class="ghost" :disabled="!dirty" @click="revert">Revert</button>
        <button class="primary" :disabled="!dirty || saving" @click="save">
          {{ saving ? "Saving…" : "Save" }}
        </button>
      </div>
    </header>

    <div class="content-body config-layout">
      <aside class="config-list">
        <button
          v-for="c in configs"
          :key="c.relative_path"
          class="config-list-item"
          :class="{ active: c.relative_path === activePath }"
          @click="activePath = c.relative_path"
        >
          <div class="cli-name">{{ c.name }}</div>
          <div class="cli-path">{{ c.relative_path }}</div>
          <div class="cli-desc">{{ descriptionFor(c.name) }}</div>
        </button>
      </aside>

      <div class="config-editor">
        <div v-if="activeConfig" class="editor-meta">
          <span class="badge info">{{ activeConfig.name }}</span>
          <span class="badge">{{ (activeConfig.size / 1024).toFixed(2) }} KB</span>
          <span v-if="dirty" class="badge warn">unsaved</span>
        </div>
        <textarea
          v-model="content"
          class="editor-text"
          spellcheck="false"
          :placeholder="activeConfig ? `Edit ${activeConfig.name}…` : 'Select a config file'"
        />
      </div>
    </div>
  </section>
</template>

<style scoped>
.config-layout {
  display: grid;
  grid-template-columns: 260px 1fr;
  gap: 16px;
  height: calc(100vh - 110px);
}

.config-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  overflow: auto;
}

.config-list-item {
  text-align: left;
  background: var(--bg-1);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 10px 12px;
  cursor: pointer;
  transition: border-color var(--transition), background var(--transition);
}
.config-list-item:hover {
  background: var(--bg-2);
}
.config-list-item.active {
  border-color: var(--accent);
  background: var(--accent-soft);
}
.cli-name {
  font-weight: 600;
  font-size: 13px;
  font-family: var(--font-mono);
  color: var(--text-0);
}
.cli-path {
  font-size: 11px;
  color: var(--text-2);
  margin-top: 2px;
}
.cli-desc {
  font-size: 11.5px;
  color: var(--text-2);
  margin-top: 6px;
  line-height: 1.4;
}

.config-editor {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-height: 0;
}

.editor-meta {
  display: flex;
  align-items: center;
  gap: 6px;
}

.editor-text {
  flex: 1;
  min-height: 0;
  height: 100%;
  background: var(--bg-0);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 12px 14px;
  font-family: var(--font-mono);
  font-size: 12.5px;
  line-height: 1.6;
  color: var(--text-0);
  resize: none;
  outline: none;
  tab-size: 2;
  white-space: pre;
  overflow: auto;
}
.editor-text:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-soft);
}
</style>
