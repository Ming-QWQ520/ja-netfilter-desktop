<script setup lang="ts">
import { useToastItems, toast } from "@/stores/toast";
const items = useToastItems();
</script>

<template>
  <div class="toast-stack">
    <div
      v-for="t in items"
      :key="t.id"
      class="toast"
      :class="t.kind"
      @click="toast.dismiss(t.id)"
    >
      {{ t.text }}
    </div>
  </div>
</template>
