import { defineStore } from "pinia";
import { ref } from "vue";

import { api } from "@/api";
import type { ProductInfo } from "@/types";

export const useProductsStore = defineStore("products", () => {
  const items = ref<ProductInfo[]>([]);
  const loading = ref(false);
  const lastError = ref<string | null>(null);

  async function refreshAll() {
    loading.value = true;
    lastError.value = null;
    try {
      items.value = await api.listProducts();
    } catch (e: any) {
      lastError.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function refreshOne(id: string) {
    try {
      const updated = await api.refreshProductStatus(id);
      const idx = items.value.findIndex((p) => p.id === id);
      if (idx >= 0) items.value[idx] = updated;
      else items.value.push(updated);
    } catch (e: any) {
      lastError.value = String(e);
    }
  }

  async function install(id: string, licenseName?: string) {
    await api.installProduct(id, licenseName);
    await refreshOne(id);
  }

  async function uninstall(id: string) {
    await api.uninstallProduct(id);
    await refreshOne(id);
  }

  async function installAll(licenseName?: string) {
    await api.installAllProducts(licenseName);
    await refreshAll();
  }

  async function uninstallAll() {
    await api.uninstallAllProducts();
    await refreshAll();
  }

  return {
    items,
    loading,
    lastError,
    refreshAll,
    refreshOne,
    install,
    uninstall,
    installAll,
    uninstallAll,
  };
});
