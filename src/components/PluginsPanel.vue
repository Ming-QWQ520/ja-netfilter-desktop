<script setup lang="ts">
import { onMounted, ref } from "vue";

import { api } from "@/api";
import { useToast } from "@/stores/toast";
import type { PluginJar } from "@/types";

const toast = useToast();
const plugins = ref<PluginJar[]>([]);
const loading = ref(false);

const pluginDocs: Record<string, string> = {
  "dns.jar": "Hook for `java.net.InetAddress` lookups. Matches hostnames against dns.conf rules — block, fall-through, or rewrite.",
  "hideme.jar": "Hides the ja-netfilter agent from `Instrumentation.getInitiatedClasses()` so IDE-side anti-cheat checks don't see it.",
  "power.jar": "Implements the transformation engine from power.conf — used to swap RSA modulus / exponent values inside the IDE.",
  "url.jar": "Intercepts HTTP requests matching url.conf rules and serves canned responses locally instead of hitting JetBrains servers.",
};

function descriptionFor(name: string): string {
  return pluginDocs[name] ?? "ja-netfilter plugin.";
}

async function load() {
  loading.value = true;
  try {
    plugins.value = await api.readPluginJars();
  } catch (e: any) {
    toast.error(String(e));
  } finally {
    loading.value = false;
  }
}

onMounted(load);
</script>

<template>
  <section class="content-area">
    <header class="content-header">
      <div>
        <h2>Plugins</h2>
        <p class="subtitle">java agent plugins loaded by lib.jar at IDE startup.</p>
      </div>
      <div class="toolbar">
        <button class="ghost" :disabled="loading" @click="load">Reload</button>
      </div>
    </header>

    <div class="content-body">
      <div v-if="loading && !plugins.length" class="empty">Loading plugins…</div>
      <div v-else-if="!plugins.length" class="empty">No plugins found.</div>
      <div v-else class="grid-cards">
        <article v-for="p in plugins" :key="p.path" class="plugin-card">
          <header>
            <div class="plugin-name">{{ p.name }}</div>
            <span class="badge info">{{ (p.size / 1024).toFixed(1) }} KB</span>
          </header>
          <p class="plugin-desc">{{ descriptionFor(p.name) }}</p>
          <div class="plugin-path mono" :title="p.path">{{ p.path }}</div>
        </article>
      </div>
    </div>
  </section>
</template>

<style scoped>
.empty {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-2);
  font-size: 13px;
}

.plugin-card {
  background: var(--bg-1);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 14px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.plugin-card header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.plugin-name {
  font-family: var(--font-mono);
  font-weight: 600;
  font-size: 14px;
  color: var(--accent);
}
.plugin-desc {
  color: var(--text-1);
  font-size: 12.5px;
  line-height: 1.55;
  margin: 0;
}
.plugin-path {
  font-size: 11px;
  color: var(--text-2);
  word-break: break-all;
  white-space: pre-wrap;
  background: var(--bg-0);
  border: 1px solid var(--border-soft);
  border-radius: var(--radius);
  padding: 6px 8px;
}
.mono {
  font-family: var(--font-mono);
}
</style>
