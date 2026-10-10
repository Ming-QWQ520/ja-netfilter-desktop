<script setup lang="ts">
import { useConfirmState, resolveConfirm } from "@/stores/confirm";
import { useT } from "@/i18n";

const t = useT();
const state = useConfirmState();
</script>

<template>
  <Teleport to="body">
    <div v-if="state.open" class="md-dialog-scrim" @click.self="resolveConfirm(false)">
      <div class="md-dialog" role="alertdialog">
        <h3>{{ state.danger ? "⚠" : "" }} {{ t("confirm_title") }}</h3>
        <p>{{ state.message }}</p>
        <div class="md-dialog__actions">
          <button class="md-btn md-btn--text" @click="resolveConfirm(false)">
            {{ t("action_cancel") }}
          </button>
          <button
            class="md-btn"
            :class="state.danger ? 'md-btn--danger-filled' : 'md-btn--filled'"
            @click="resolveConfirm(true)"
          >
            {{ t("action_continue") }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
