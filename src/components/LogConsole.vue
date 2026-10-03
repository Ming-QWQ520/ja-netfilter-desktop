<script setup lang="ts">
import { onMounted, ref } from "vue";

import { api } from "@/api";
import { useToast } from "@/stores/toast";
import type { LogLevel } from "@/types";

const toast = useToast();
const entries = ref<import("@/types").LogEntry[]>([]);
const filter = ref<LogLevel | "all">("all");
const autoscroll = ref(true);

const levelColor: Record<LogLevel, string> = {
  debug: "var(--text-2)",
  info: "var(--info)",
  warn: "var(--warn)",
  error: "var(--danger)",
};

const filtered = () =>
  filter.value === "all"
    ? entries.value
    : entries.value.filter((e) => e.level === filter.value);

async function load() {
  try {
    entries.value = await api.getLogHistory();
  } catch (e: any) {
    toast.error(String(e));
  }
}

async function clear() {
  await api.clearLogHistory();
  entries.value = [];
}

function copyAll() {
  const text = filtered()
    .map((e) => `[${e.ts}] ${e.level.toUpperCase()} ${e.message}`)
    .join("\n");
  navigator.clipboard.writeText(text);
  toast.success("已复制日志到剪贴板。");
}

onMounted(async () => {
  await load();
  setInterval(load, 2000);
});
</script>

<template>
  <section class="content-area">
    <header class="content-header">
      <div>
        <h2>日志</h2>
        <p class="subtitle">所有安装 / 卸载 / 文件读写操作都会同步记录到这里。</p>
      </div>
      <div class="toolbar">
        <select v-model="filter" class="filter-select">
          <option value="all">全部级别</option>
          <option value="debug">调试</option>
          <option value="info">信息</option>
          <option value="warn">警告</option>
          <option value="error">错误</option>
        </select>
        <label class="check">
          <input type="checkbox" v-model="autoscroll" /> 自动滚动
        </label>
        <button class="ghost" @click="copyAll">复制</button>
        <button class="danger" @click="clear">清空</button>
        <button class="ghost" @click="load">刷新</button>
      </div>
    </header>

    <div class="content-body">
      <div class="log-stream">
        <div v-for="(e, i) in filtered()" :key="i" class="log-line">
          <span class="log-ts">{{ e.ts }}</span>
          <span class="log-level" :style="{ color: levelColor[e.level] }">{{ e.level.toUpperCase() }}</span>
          <span class="log-msg">{{ e.message }}</span>
        </div>
        <div v-if="!filtered().length" class="empty">暂无日志。</div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.filter-select {
  font-size: 12px;
  padding: 5px 8px;
}
.check {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  color: var(--text-1);
}

.log-stream {
  background: var(--bg-0);
  border: 1px solid var(--border-soft);
  border-radius: var(--radius);
  padding: 10px;
  height: calc(100vh - 130px);
  overflow: auto;
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.6;
}

.log-line {
  display: grid;
  grid-template-columns: 150px 60px 1fr;
  gap: 10px;
  padding: 2px 4px;
  border-radius: 4px;
}
.log-line:hover {
  background: var(--bg-1);
}
.log-ts {
  color: var(--text-3);
}
.log-level {
  font-weight: 600;
  letter-spacing: 0.04em;
}
.log-msg {
  color: var(--text-0);
  word-break: break-word;
  white-space: pre-wrap;
}

.empty {
  color: var(--text-2);
  text-align: center;
  padding: 40px 0;
}
</style>
