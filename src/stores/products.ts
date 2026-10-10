import { defineStore } from "pinia";
import { ref } from "vue";

import { api } from "@/api";
import type { InstallResult, ProductInfo } from "@/types";

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

  /** 安装单个产品（含自定义授权），返回后端结果。 */
  async function install(
    id: string,
    licenseName?: string,
    licenseExpiry?: string,
  ): Promise<InstallResult> {
    const result = await api.installProduct(id, licenseName, licenseExpiry);
    await refreshOne(id);
    return result;
  }

  /** 卸载单个产品。 */
  async function uninstall(id: string): Promise<InstallResult> {
    const result = await api.uninstallProduct(id);
    await refreshOne(id);
    return result;
  }

  /** 全部安装（单次后端调用覆盖所有产品，含自定义授权）。 */
  async function installAll(
    licenseName?: string,
    licenseExpiry?: string,
  ): Promise<InstallResult[]> {
    const results = await api.installAllProducts(licenseName, licenseExpiry);
    await refreshAll();
    return results;
  }

  /** 全部卸载。 */
  async function uninstallAll(): Promise<InstallResult[]> {
    const results = await api.uninstallAllProducts();
    await refreshAll();
    return results;
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
