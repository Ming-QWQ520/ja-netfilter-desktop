<script setup lang="ts">
import { onMounted, ref } from "vue";

import { api } from "@/api";
import { useToast } from "@/stores/toast";
import type { PluginJar } from "@/types";

const toast = useToast();
const plugins = ref<PluginJar[]>([]);
const loading = ref(false);

const pluginDocs: Record<string, string> = {
  "dns.jar": "Hook `java.net.InetAddress` 的域名查询。按 dns.conf 规则匹配主机名 —— 阻断、放行或改写。",
  "hideme.jar": "对 `Instrumentation.getInitiatedClasses()` 隐藏 ja-netfilter 自身，避免 IDE 内部的探测检查发现 agent。",
  "power.jar": "实现 power.conf 中描述的变换引擎 —— 用于替换 IDE 内部 RSA 模数 / 指数等敏感值。",
  "url.jar": "拦截匹配 url.conf 规则的 HTTP 请求，并直接返回本地响应，无需访问 JetBrains 服务器。",
};

function descriptionFor(name: string): string {
  return pluginDocs[name] ?? "ja-netfilter 插件。";
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
        <h2>插件</h2>
        <p class="subtitle">IDE 启动时由 lib.jar 加载的 Java agent 插件。</p>
      </div>
      <div class="toolbar">
        <button class="ghost" :disabled="loading" @click="load">重新加载</button>
      </div>
    </header>

    <div class="content-body">
      <div v-if="loading && !plugins.length" class="empty">正在加载插件列表…</div>
      <div v-else-if="!plugins.length" class="empty">未找到任何插件。</div>
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
