import { defineStore } from "pinia";
import { ref } from "vue";

import { api } from "@/api";
import type { LogEntry, LogLevel } from "@/types";

export const useLogsStore = defineStore("logs", () => {
  const entries = ref<LogEntry[]>([]);
  const filter = ref<LogLevel | "all">("all");

  async function refresh() {
    try {
      entries.value = await api.getLogHistory();
    } catch {
      // ignore; logs are best-effort
    }
  }

  async function clear() {
    await api.clearLogHistory();
    entries.value = [];
  }

  function push(entry: LogEntry) {
    entries.value.push(entry);
    if (entries.value.length > 1024) entries.value.shift();
  }

  function setFilter(level: LogLevel | "all") {
    filter.value = level;
  }

  return { entries, filter, refresh, clear, push, setFilter };
});
