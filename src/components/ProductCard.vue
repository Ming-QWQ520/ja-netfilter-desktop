<script setup lang="ts">
import { ref } from "vue";

import { useProductsStore } from "@/stores/products";
import { useLicenseStore } from "@/stores/license";
import { useToast } from "@/stores/toast";
import type { ProductInfo, VmoptionsSource } from "@/types";

const props = defineProps<{ product: ProductInfo }>();

const store = useProductsStore();
const license = useLicenseStore();
const toast = useToast();

const busy = ref(false);
const expanded = ref(false);

async function install() {
  busy.value = true;
  try {
    await store.install(props.product.id, license.name || undefined);
    toast.success(`${props.product.name}：javaagent 已安装。`);
  } catch (e: any) {
    toast.error(`${props.product.name}：${e}`);
  } finally {
    busy.value = false;
  }
}

async function uninstall() {
  busy.value = true;
  try {
    await store.uninstall(props.product.id);
    toast.success(`${props.product.name}：javaagent 已移除。`);
  } catch (e: any) {
    toast.error(`${props.product.name}：${e}`);
  } finally {
    busy.value = false;
  }
}

function sourceLabel(src: VmoptionsSource): string {
  return {
    env: "环境变量",
    ide: "IDE 自带",
    user: "用户配置",
    template: "项目模板",
    missing: "缺失",
  }[src];
}

function sourceClass(src: VmoptionsSource): string {
  return {
    env: "ok",
    ide: "ok",
    user: "ok",
    template: "warn",
    missing: "error",
  }[src];
}
</script>

<template>
  <article class="product-card" :class="{ installed: product.javaagent_installed }">
    <header class="card-header">
      <div class="card-title">
        <span class="dot" :class="product.javaagent_installed ? 'ok' : 'warn'" />
        <span class="name">{{ product.name }}</span>
      </div>
      <span class="badge" :class="product.javaagent_installed ? 'ok' : ''">
        {{ product.javaagent_installed ? "已安装" : "未安装" }}
      </span>
    </header>

    <dl class="card-meta">
      <div>
        <dt>标识</dt>
        <dd class="mono">{{ product.id }}</dd>
      </div>
      <div>
        <dt>环境变量</dt>
        <dd class="mono">{{ product.env_var }}</dd>
      </div>
      <div>
        <dt>vmoptions</dt>
        <dd class="mono small">
          <span :title="product.vmoptions_path ?? ''">
            {{ product.vmoptions_path ?? "—" }}
          </span>
        </dd>
      </div>
      <div>
        <dt>来源</dt>
        <dd>
          <span class="badge" :class="sourceClass(product.vmoptions_source)">
            {{ sourceLabel(product.vmoptions_source) }}
          </span>
        </dd>
      </div>
    </dl>

    <div v-if="product.javaagent_target" class="javaagent-line">
      <code>{{ product.javaagent_target }}</code>
    </div>

    <footer class="card-actions">
      <button class="ghost" @click="expanded = !expanded">
        {{ expanded ? "收起预览" : "预览 vmoptions" }}
      </button>
      <div class="spacer" />
      <button class="danger" :disabled="busy || !product.javaagent_installed" @click="uninstall">
        卸载
      </button>
      <button class="primary" :disabled="busy" @click="install">
        {{ busy ? "…" : product.javaagent_installed ? "重新安装" : "安装" }}
      </button>
    </footer>

    <pre v-if="expanded && product.vmoptions_preview" class="vmoptions-preview">{{ product.vmoptions_preview }}</pre>
  </article>
</template>

<style scoped>
.product-card {
  background: var(--bg-1);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  transition: border-color var(--transition), box-shadow var(--transition);
}
.product-card:hover {
  border-color: var(--bg-3);
  box-shadow: var(--shadow-1);
}
.product-card.installed {
  border-left: 2px solid var(--success);
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.card-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  font-size: 14px;
}

.card-meta {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px 14px;
  margin: 0;
  font-size: 12px;
}
.card-meta dt {
  color: var(--text-2);
  font-weight: 500;
  margin-bottom: 2px;
}
.card-meta dd {
  margin: 0;
  color: var(--text-0);
}
.mono {
  font-family: var(--font-mono);
  font-size: 12px;
}
.mono.small {
  font-size: 11px;
  word-break: break-all;
  white-space: pre-wrap;
}

.javaagent-line {
  background: var(--bg-2);
  border: 1px solid var(--border-soft);
  border-radius: var(--radius);
  padding: 6px 8px;
  font-size: 11.5px;
  color: var(--success);
}
.javaagent-line code {
  font-family: var(--font-mono);
  word-break: break-all;
  white-space: pre-wrap;
}

.card-actions {
  display: flex;
  align-items: center;
  gap: 6px;
}
.card-actions .spacer {
  flex: 1;
}

.vmoptions-preview {
  background: var(--bg-0);
  border: 1px solid var(--border-soft);
  border-radius: var(--radius);
  padding: 10px;
  font-family: var(--font-mono);
  font-size: 11.5px;
  line-height: 1.55;
  color: var(--text-1);
  max-height: 220px;
  overflow: auto;
  margin: 0;
}
</style>
