import { ref } from "vue";

export interface ConfirmState {
  open: boolean;
  message: string;
  danger: boolean;
  resolve: ((ok: boolean) => void) | null;
}

const state = ref<ConfirmState>({
  open: false,
  message: "",
  danger: false,
  resolve: null,
});

/** Promise 化确认对话框：await confirmDialog("…") === true/false */
export function confirmDialog(message: string, danger = false): Promise<boolean> {
  return new Promise((resolve) => {
    state.value = { open: true, message, danger, resolve };
  });
}

export function resolveConfirm(ok: boolean) {
  state.value.resolve?.(ok);
  state.value.open = false;
}

export function useConfirmState() {
  return state;
}
