<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";

import { api } from "@/api";
import { useToast } from "@/stores/toast";
import { useProductsStore } from "@/stores/products";

const toast = useToast();
const store = useProductsStore();

const selectedId = ref<string | null>(null);
const content = ref("");
const originalContent = ref("");
const loading = ref(false);
const saving = ref(false);

const dirty = computed(() => content.value !== originalContent.value);

const sortedProducts = computed(() =>
  [...store.items].sort((a, b) => a.name.localeCompare(b.name)),
);

async function loadContent(id: string) {
  loading.value = true;
  try {
    const text = await api.readVmoptions(id);
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
  if (!selectedId.value) return;
  saving.value = true;
  try {
    await api.writeVmoptions(selectedId.value, content.value);
    originalContent.value = content.value;
    toast.success(`已保存 ${selectedId.value}.vmoptions`);
  } catch (e: any) {
    toast.error(String(e));
  } finally {
    saving.value = false;
  }
}

function revert() {
  content.value = originalContent.value;
}

async function resetTemplate() {
  if (!selectedId.value) return;
  if (!confirm(`确定要将 ${selectedId.value}.vmoptions 重置为项目自带模板吗？`)) return;
  try {
    await api.resetVmoptions(selectedId.value);
    await loadContent(selectedId.value);
    toast.success("已重置为项目模板。");
  } catch (e: any) {
    toast.error(String(e));
  }
}

watch(selectedId, async (id) => {
  if (id) await loadContent(id);
});

onMounted(async () => {
  if (!store.items.length) await store.refreshAll();
  if (store.items.length && !selectedId.value) {
    selectedId.value = store.items[0].id;
  }
});
</script>

<template>
  <section class="content-area">
    <header class="content-header">
      <div>
        <h2>vmoptions</h2>
        <p class="subtitle">查看或编辑项目自带的各产品 vmoptions 模板。</p>
      </div>
      <div class="toolbar">
        <button class="ghost" :disabled="!dirty" @click="revert">撤销</button>
        <button class="danger" :disabled="!selectedId" @click="resetTemplate">重置为模板</button>
        <button class="primary" :disabled="!dirty || saving" @click="save">
          {{ saving ? "保存中…" : "保存" }}
        </button>
      </div>
    </header>

    <div class="content-body config-layout">
      <aside class="vm-list">
        <button
          v-for="p in sortedProducts"
          :key="p.id"
          class="vm-list-item"
          :class="{ active: p.id === selectedId }"
          @click="selectedId = p.id"
        >
          <span class="dot" :class="p.javaagent_installed ? 'ok' : 'warn'" />
          <span class="vm-name">{{ p.name }}</span>
          <span class="vm-id">{{ p.id }}</span>
        </button>
      </aside>

      <div class="vm-editor">
        <div v-if="selectedId" class="editor-meta">
          <span class="badge info">{{ selectedId }}.vmoptions</span>
          <span v-if="dirty" class="badge warn">未保存</span>
          <span v-else class="badge ok">已同步</span>
        </div>
        <textarea
          v-model="content"
          class="editor-text"
          spellcheck="false"
          :placeholder="selectedId ? `编辑 ${selectedId}.vmoptions…` : '请选择一个产品'"
        />
      </div>
    </div>
  </section>
</template>

<style scoped>
.config-layout {
  display: grid;
  grid-template-columns: 280px 1fr;
  gap: 16px;
  height: calc(100vh - 110px);
}

.vm-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  overflow: auto;
}

.vm-list-item {
  display: grid;
  grid-template-columns: 12px 1fr auto;
  align-items: center;
  gap: 8px;
  text-align: left;
  background: var(--bg-1);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 8px 10px;
  cursor: pointer;
  transition: border-color var(--transition), background var(--transition);
}
.vm-list-item:hover {
  background: var(--bg-2);
}
.vm-list-item.active {
  border-color: var(--accent);
  background: var(--accent-soft);
}

.vm-name {
  font-weight: 500;
  font-size: 13px;
  color: var(--text-0);
}
.vm-id {
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--text-2);
}

.vm-editor {
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
