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
        <div class="dialog-badge" :class="{ danger: state.danger }" aria-hidden="true">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <template v-if="state.danger">
              <path d="M12 8v5" /><circle cx="12" cy="16.2" r="0.6" fill="currentColor" stroke="none" /><path d="M10.3 3.9 2.8 17a2 2 0 0 0 1.7 3h15a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z" />
            </template>
            <template v-else>
              <circle cx="12" cy="12" r="9" /><path d="M12 8v4" /><circle cx="12" cy="16" r="0.6" fill="currentColor" stroke="none" />
            </template>
          </svg>
        </div>
        <h3>{{ t("confirm_title") }}</h3>
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

<style scoped>
.dialog-badge {
  width: 38px;
  height: 38px;
  border-radius: var(--md-corner-m);
  background: var(--md-primary-container);
  color: var(--md-on-primary-container);
  display: grid;
  place-content: center;
  margin-bottom: 12px;
}
.dialog-badge svg {
  width: 20px;
  height: 20px;
}
.dialog-badge.danger {
  background: var(--md-error-container);
  color: var(--md-on-error-container);
}
.md-dialog h3 {
  margin: 0 0 8px;
}
.md-dialog p {
  margin: 0 0 18px;
}
</style>
