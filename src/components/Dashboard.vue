<script setup lang="ts">
import { onMounted, ref } from "vue";

import ProductCard from "@/components/ProductCard.vue";
import { useProductsStore } from "@/stores/products";
import { useLicenseStore } from "@/stores/license";
import { useToast } from "@/stores/toast";

const store = useProductsStore();
const license = useLicenseStore();
const toast = useToast();

const installingAll = ref(false);

async function installAll() {
  installingAll.value = true;
  try {
    await store.installAll(license.name || undefined);
    toast.success("已为全部产品安装 javaagent。");
  } catch (e: any) {
    toast.error(String(e));
  } finally {
    installingAll.value = false;
  }
}

async function uninstallAll() {
  installingAll.value = true;
  try {
    await store.uninstallAll();
    toast.success("已从全部产品移除 javaagent。");
  } catch (e: any) {
    toast.error(String(e));
  } finally {
    installingAll.value = false;
  }
}

const installedCount = () => store.items.filter((p) => p.javaagent_installed).length;
const totalCount = () => store.items.length;

onMounted(() => {
  store.refreshAll();
});
</script>

<template>
  <section class="content-area">
    <header class="content-header">
      <div>
        <h2>总览</h2>
        <p class="subtitle">为已安装的 JetBrains IDE 统一管理 ja-netfilter javaagent。</p>
      </div>
      <div class="toolbar">
        <span class="badge ok">
          <span class="dot ok" />
          已安装 {{ installedCount() }} / {{ totalCount() }}
        </span>
        <button class="ghost" :disabled="store.loading" @click="store.refreshAll()">
          刷新
        </button>
        <button class="danger" :disabled="installingAll || store.loading" @click="uninstallAll">
          全部卸载
        </button>
        <button class="primary" :disabled="installingAll || store.loading" @click="installAll">
          {{ installingAll ? "正在处理…" : "全部安装" }}
        </button>
      </div>
    </header>

    <div class="content-body">
      <div v-if="store.loading && !store.items.length" class="empty">
        <p>正在检测已安装的 JetBrains 产品…</p>
      </div>
      <div v-else-if="!store.items.length" class="empty">
        <p>未检测到任何产品。</p>
      </div>
      <div v-else class="grid-cards">
        <ProductCard
          v-for="product in store.items"
          :key="product.id"
          :product="product"
        />
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
</style>
