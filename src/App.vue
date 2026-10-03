<script setup lang="ts">
import { onMounted, ref } from "vue";
import { listen } from "@tauri-apps/api/event";

import Sidebar from "@/components/Sidebar.vue";
import ToastHost from "@/components/ToastHost.vue";
import { useProductsStore } from "@/stores/products";
import { useSettingsStore } from "@/stores/settings";
import { useLogsStore } from "@/stores/logs";
import type { LogEntry } from "@/types";

const productsStore = useProductsStore();
const settingsStore = useSettingsStore();
const logsStore = useLogsStore();

const ready = ref(false);

onMounted(async () => {
  // Wire up the live log stream
  await logsStore.refresh();
  await listen<LogEntry>("log://entry", (event) => {
    logsStore.push(event.payload);
  });

  await Promise.all([
    productsStore.refreshAll(),
    settingsStore.refresh(),
  ]);
  ready.value = true;
});
</script>

<template>
  <div class="app-shell">
    <Sidebar />
    <router-view v-if="ready" v-slot="{ Component }">
      <component :is="Component" />
    </router-view>
    <div v-else class="loading-screen">
      <div class="loader" />
      <p>正在加载工作区…</p>
    </div>
    <ToastHost />
  </div>
</template>

<style scoped>
.loading-screen {
  grid-column: 2;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 14px;
  color: var(--text-2);
}
.loader {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  border: 2px solid var(--bg-3);
  border-top-color: var(--accent);
  animation: spin 800ms linear infinite;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
