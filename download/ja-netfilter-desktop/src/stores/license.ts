import { defineStore } from "pinia";
import { ref, watch } from "vue";

/**
 * 自定义授权信息（仿 ckey_script.ps1 的 Read_Host_License_Info）：
 *   - 授权名称：回车留空默认 "Ming"
 *   - 到期时间：回车留空默认 "2099-12-31"（格式 yyyy-MM-dd）
 * 两个值随安装命令传给后端，由 ckey.run 生成 `<prd>.key` 授权文件。
 * 持久化到 localStorage，只需配置一次。
 */

export const DEFAULT_LICENSE_NAME = "Ming";
export const DEFAULT_EXPIRY_DATE = "2099-12-31";

const NAME_KEY = "ja-netfilter-desktop:licenseName";
const EXPIRY_KEY = "ja-netfilter-desktop:licenseExpiry";

export function isValidExpiry(s: string): boolean {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(s)) return false;
  const d = new Date(`${s}T00:00:00Z`);
  return !Number.isNaN(d.getTime()) && d.toISOString().startsWith(s);
}

export const useLicenseStore = defineStore("license", () => {
  const name = ref<string>(localStorage.getItem(NAME_KEY) ?? DEFAULT_LICENSE_NAME);
  const expiry = ref<string>(localStorage.getItem(EXPIRY_KEY) ?? DEFAULT_EXPIRY_DATE);

  function setName(value: string) {
    // 回车留空 → 默认 Ming（与 ckey_script 的 Read-Host 语义一致）
    const v = value.trim();
    name.value = v || DEFAULT_LICENSE_NAME;
  }

  function setExpiry(value: string) {
    // 留空或非法 → 默认 2099-12-31（与 ckey_script 的 Read-Valid_Date 语义一致）
    const v = value.trim();
    expiry.value = isValidExpiry(v) ? v : DEFAULT_EXPIRY_DATE;
  }

  watch(
    name,
    (val) => {
      if (val && val !== DEFAULT_LICENSE_NAME) localStorage.setItem(NAME_KEY, val);
      else localStorage.removeItem(NAME_KEY);
    },
    { immediate: true },
  );

  watch(
    expiry,
    (val) => {
      if (val && val !== DEFAULT_EXPIRY_DATE) localStorage.setItem(EXPIRY_KEY, val);
      else localStorage.removeItem(EXPIRY_KEY);
    },
    { immediate: true },
  );

  return { name, expiry, setName, setExpiry };
});
