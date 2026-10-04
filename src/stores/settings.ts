import { defineStore } from "pinia";
import { ref } from "vue";

import { api } from "@/api";
import type { WorkspaceInfo } from "@/types";

export const useSettingsStore = defineStore("settings", () => {
  const info = ref<WorkspaceInfo | null>(null);
  const loadError = ref<string | null>(null);

  async function refresh() {
    try {
      info.value = await api.getWorkspaceInfo();
    } catch (e: any) {
      loadError.value = String(e);
    }
  }

  async function reveal(path: string) {
    try {
      await api.revealInFinder(path);
    } catch (e: any) {
      loadError.value = String(e);
    }
  }

  return { info, loadError, refresh, reveal };
});
