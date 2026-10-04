import { defineStore } from "pinia";
import { ref, watch } from "vue";

const STORAGE_KEY = "ja-netfilter-desktop:licenseName";

export const useLicenseStore = defineStore("license", () => {
  // The custom license name injected into vmoptions via
  //   -Dja.netfilter.name=<value>
  // Persisted to localStorage so the user only configures it once.
  const name = ref<string>(localStorage.getItem(STORAGE_KEY) ?? "");

  function set(value: string) {
    name.value = value.trim();
  }

  watch(
    name,
    (val) => {
      if (val) localStorage.setItem(STORAGE_KEY, val);
      else localStorage.removeItem(STORAGE_KEY);
    },
    { immediate: true },
  );

  return { name, set };
});
