<script setup lang="ts">
import { onMounted, ref } from "vue";

import ProductCard from "@/components/ProductCard.vue";
import { useProductsStore } from "@/stores/products";
import { useToast } from "@/stores/toast";

const store = useProductsStore();
const toast = useToast();

const installingAll = ref(false);

async function installAll() {
  installingAll.value = true;
  try {
    await store.installAll();
    toast.success("Install all done.");
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
    toast.success("Uninstall all done.");
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
        <h2>Dashboard</h2>
        <p class="subtitle">Manage the ja-netfilter javaagent across every installed JetBrains IDE.</p>
      </div>
      <div class="toolbar">
        <span class="badge ok">
          <span class="dot ok" />
          {{ installedCount() }} / {{ totalCount() }} installed
        </span>
        <button class="ghost" :disabled="store.loading" @click="store.refreshAll()">
          Refresh
        </button>
        <button class="danger" :disabled="installingAll || store.loading" @click="uninstallAll">
          Uninstall all
        </button>
        <button class="primary" :disabled="installingAll || store.loading" @click="installAll">
          {{ installingAll ? "Working…" : "Install all" }}
        </button>
      </div>
    </header>

    <div class="content-body">
      <div v-if="store.loading && !store.items.length" class="empty">
        <p>Detecting installed JetBrains products…</p>
      </div>
      <div v-else-if="!store.items.length" class="empty">
        <p>No products detected.</p>
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
