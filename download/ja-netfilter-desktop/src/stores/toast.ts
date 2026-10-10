import { ref } from "vue";

export type ToastKind = "info" | "success" | "warn" | "error";

export interface ToastItem {
  id: number;
  kind: ToastKind;
  text: string;
}

const items = ref<ToastItem[]>([]);
let counter = 0;

function push(kind: ToastKind, text: string) {
  const id = ++counter;
  items.value.push({ id, kind, text });
  // MD3 snackbar 自动关闭（错误保留更久）
  const ttl = kind === "error" ? 6500 : 3800;
  setTimeout(() => dismiss(id), ttl);
}

function dismiss(id: number) {
  items.value = items.value.filter((t) => t.id !== id);
}

export const toast = {
  info: (t: string) => push("info", t),
  success: (t: string) => push("success", t),
  warn: (t: string) => push("warn", t),
  error: (t: string) => push("error", t),
  dismiss,
};

export function useToast() {
  return toast;
}

export function useToastItems() {
  return items;
}
