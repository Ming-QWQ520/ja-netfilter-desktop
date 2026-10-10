<script setup lang="ts">
import { computed, onMounted, ref, watch, nextTick, onUnmounted } from "vue";
import { listen } from "@tauri-apps/api/event";

import { api } from "@/api";
import { useT } from "@/i18n";
import { useLogsStore } from "@/stores/logs";
import { useToast } from "@/stores/toast";
import type { LogEntry, LogLevel } from "@/types";

const t = useT();
const toast = useToast();
const store = useLogsStore();

const autoscroll = ref(true);
const streamEl = ref<HTMLElement | null>(null);
const unlisten = ref<(() => void) | null>(null);

const filters: { id: LogLevel | "all"; labelKey: string }[] = [
  { id: "all", labelKey: "log_all" },
  { id: "info", labelKey: "log_info" },
  { id: "success", labelKey: "log_success" },
  { id: "warn", labelKey: "log_warn" },
  { id: "error", labelKey: "log_error" },
  { id: "debug", labelKey: "log_debug" },
];

const levelColor: Record<LogLevel, string> = {
  debug: "var(--md-outline)",
  info: "var(--md-tertiary)",
  success: "var(--md-success)",
  warn: "var(--md-warning)",
  error: "var(--md-error)",
};

const filtered = computed(() =>
  store.filter === "all"
    ? store.entries
    : store.entries.filter((e) => e.level === store.filter),
);

async function scrollToBottom() {
  if (!autoscroll.value) return;
  await nextTick();
  streamEl.value?.scrollTo({ top: streamEl.value.scrollHeight });
}

function copyAll() {
  const text = filtered.value
    .map((e) => `[${e.ts}][${e.level.toUpperCase()}] ${e.message}`)
    .join("\n");
  navigator.clipboard.writeText(text);
  toast.success(t("copied"));
}

async function clearAll() {
  await api.clearLogHistory();
  store.entries = [];
  toast.info(t("logs_cleared"));
}

onMounted(async () => {
  await store.refresh();
  // Rust 端事件推送（v0.2.0 修复：旧版从未发出）
  unlisten.value = await listen<LogEntry>("log://entry", (event) => {
    store.push(event.payload);
    scrollToBottom();
  });
});

onUnmounted(() => {
  unlisten.value?.();
});

watch(
  () => store.entries.length,
  () => scrollToBottom(),
);
</script>

<template>
  <section class="tab-panel">
    <div class="logs-toolbar">
      <div>
        <h2>{{ t("tab_logs") }}</h2>
        <p class="desc">{{ t("logs_desc") }}</p>
      </div>
      <div class="toolbar-right">
        <div class="filter-chips">
          <button
            v-for="f in filters"
            :key="f.id"
            class="md-chip md-chip--filter"
            :class="{ 'is-selected': store.filter === f.id }"
            @click="store.setFilter(f.id)"
          >
            {{ t(f.labelKey) }}
          </button>
        </div>
      </div>
    </div>

    <div class="logs-meta">
      <label class="switch-row">
        <span>{{ t("autoscroll") }}</span>
        <span class="md-switch">
          <input type="checkbox" v-model="autoscroll" />
          <span class="md-switch__track" />
          <span class="md-switch__thumb" />
        </span>
      </label>
      <div class="meta-actions">
        <button class="md-btn md-btn--text md-btn--sm" @click="copyAll">{{ t("copy") }}</button>
        <button class="md-btn md-btn--danger-outlined md-btn--sm" @click="clearAll">
          {{ t("clear") }}
        </button>
      </div>
    </div>

    <div ref="streamEl" class="log-stream mono selectable">
      <div v-for="(e, i) in filtered" :key="i" class="log-line">
        <span class="log-ts">{{ e.ts }}</span>
        <span class="log-level" :style="{ color: levelColor[e.level] }">
          {{ e.level.toUpperCase() }}
        </span>
        <span class="log-msg">{{ e.message }}</span>
      </div>
      <div v-if="!filtered.length" class="md-empty">{{ t("no_logs") }}</div>
    </div>
  </section>
</template>

<style scoped>
.tab-panel {
  height: 100%;
  overflow: hidden;
  padding: 20px 24px 24px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.logs-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
}
.logs-toolbar h2 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
}
.desc {
  margin: 2px 0 0;
  font-size: 12.5px;
  color: var(--md-on-surface-variant);
}
.filter-chips {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.logs-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.switch-row {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  font-size: 13px;
  color: var(--md-on-surface-variant);
  cursor: pointer;
}
.meta-actions {
  display: flex;
  gap: 6px;
}

.log-stream {
  flex: 1;
  min-height: 0;
  overflow: auto;
  background: var(--md-surface-container-lowest);
  box-shadow: inset 0 0 0 1px var(--md-outline-variant);
  border-radius: var(--md-corner-m);
  padding: 12px;
  font-size: 12px;
  line-height: 1.7;
}
.log-line {
  display: grid;
  grid-template-columns: 150px 70px 1fr;
  gap: 10px;
  padding: 1px 6px;
  border-radius: var(--md-corner-xs);
}
.log-line:hover {
  background: var(--md-surface-container);
}
.log-ts {
  color: var(--md-outline);
}
.log-level {
  font-weight: 600;
  letter-spacing: 0.05em;
}
.log-msg {
  color: var(--md-on-surface);
  word-break: break-word;
  white-space: pre-wrap;
}
</style>
