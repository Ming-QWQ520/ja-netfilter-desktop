<script setup lang="ts">
import { useToastItems, toast } from "@/stores/toast";
const items = useToastItems();
</script>

<template>
  <div class="md-snackbar-host">
    <div
      v-for="item in items"
      :key="item.id"
      class="md-snackbar"
      :class="`md-snackbar--${item.kind}`"
      @click="toast.dismiss(item.id)"
      role="status"
    >
      {{ item.text }}
    </div>
  </div>
</template>
