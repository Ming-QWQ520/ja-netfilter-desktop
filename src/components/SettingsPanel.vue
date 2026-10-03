<script setup lang="ts">
import { computed, onMounted, ref } from "vue";

import { useSettingsStore } from "@/stores/settings";
import { useLicenseStore } from "@/stores/license";
import { useToast } from "@/stores/toast";
import { api } from "@/api";

const settings = useSettingsStore();
const license = useLicenseStore();
const toast = useToast();

const info = computed(() => settings.info);
const licenseInput = ref(license.name);

// 插件目录：与 lib.jar 同级，目录名 plugins-jetbrains
// lib.jar 路径形如 <resource_root>/lib.jar，去掉末尾的 lib.jar 后拼接 plugins-jetbrains
const pluginsDir = computed(() => {
  const jarPath = info.value?.jar_path ?? "";
  if (!jarPath) return "";
  // 去掉末尾的 /lib.jar 或 \lib.jar
  const base = jarPath.replace(/[/\\]lib\.jar$/, "");
  return `${base}/plugins-jetbrains`;
});

const osLabel = computed(() => {
  switch (info.value?.os) {
    case "windows":
      return "Windows";
    case "macos":
      return "macOS";
    case "linux":
      return "Linux";
    default:
      return "未知";
  }
});

async function reveal(path: string) {
  try {
    await settings.reveal(path);
  } catch (e: any) {
    toast.error(String(e));
  }
}

async function pickJar() {
  try {
    const path = await api.pickJarFile();
    if (path) toast.success(`已选择：${path}`);
  } catch (e: any) {
    toast.error(String(e));
  }
}

function saveLicense() {
  license.set(licenseInput.value);
  if (license.name) {
    toast.success(`自定义授权名称已保存为 “${license.name}”，下次安装时将写入 vmoptions。`);
  } else {
    toast.success("已清空自定义授权名称。");
  }
}

onMounted(() => settings.refresh());
</script>

<template>
  <section class="content-area">
    <header class="content-header">
      <div>
        <h2>设置</h2>
        <p class="subtitle">查看工作区、自带的 jar 文件、平台，并配置自定义授权名称。</p>
      </div>
      <div class="toolbar">
        <button class="ghost" @click="settings.refresh()">刷新</button>
      </div>
    </header>

    <div class="content-body">
      <!-- 自定义授权名称 -->
      <div class="setting-row license-row">
        <div class="setting-label">
          <h3>自定义授权名称</h3>
          <p>
            安装 javaagent 时，将以此名称写入
            <code>-Dja.netfilter.name=&lt;value&gt;</code>。
            留空则不写入该参数，使用 ja-netfilter 默认行为。
          </p>
        </div>
        <div class="setting-value">
          <input
            v-model="licenseInput"
            type="text"
            class="license-input"
            placeholder="例如：Ming-QWQ520"
            spellcheck="false"
          />
          <div class="actions">
            <button class="ghost" @click="licenseInput = ''">清空</button>
            <button class="primary" @click="saveLicense">保存</button>
          </div>
          <p class="hint">
            当前已保存：<code>{{ license.name || "（未设置）" }}</code>
          </p>
        </div>
      </div>

      <div v-if="!info" class="empty">加载中…</div>
      <div v-else class="settings-grid">
        <div class="setting-row">
          <div class="setting-label">
            <h3>工作区</h3>
            <p>项目自带资源（lib.jar、插件、配置、vmoptions 模板）所在的目录，应用直接读取，不做拷贝。</p>
          </div>
          <div class="setting-value">
            <code class="path">{{ info.workdir }}</code>
            <div class="actions">
              <button class="ghost" @click="reveal(info.workdir)">在文件管理器中显示</button>
            </div>
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-label">
            <h3>ja-netfilter jar</h3>
            <p>各 vmoptions 中通过 <code>-javaagent:…=jetbrains</code> 加载的 javaagent jar。</p>
          </div>
          <div class="setting-value">
            <code class="path">{{ info.jar_path }}</code>
            <div class="actions">
              <span class="badge" :class="info.jar_exists ? 'ok' : 'error'">
                {{ info.jar_exists ? "存在" : "缺失" }}
              </span>
              <button class="ghost" @click="reveal(info.jar_path)">在文件管理器中显示</button>
              <button class="ghost" @click="pickJar">选择自定义 jar…</button>
            </div>
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-label">
            <h3>插件目录</h3>
            <p>ja-netfilter 运行时查找插件 jar 的位置（与 lib.jar 同级，目录名 plugins-jetbrains 与 -javaagent:...=jetbrains 参数对应）。</p>
          </div>
          <div class="setting-value">
            <code class="path">{{ pluginsDir }}</code>
            <div class="actions">
              <button class="ghost" @click="reveal(pluginsDir)">在文件管理器中显示</button>
            </div>
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-label">
            <h3>配置目录</h3>
            <p>dns / power / url 插件启动时读取的规则文件。</p>
          </div>
          <div class="setting-value">
            <code class="path">{{ info.config_dir }}</code>
            <div class="actions">
              <button class="ghost" @click="reveal(info.config_dir)">在文件管理器中显示</button>
            </div>
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-label">
            <h3>vmoptions 模板</h3>
            <p>项目自带的各产品 vmoptions 模板，直接作为安装源使用。</p>
          </div>
          <div class="setting-value">
            <code class="path">{{ info.vmoptions_dir }}</code>
            <div class="actions">
              <button class="ghost" @click="reveal(info.vmoptions_dir)">在文件管理器中显示</button>
            </div>
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-label">
            <h3>平台</h3>
            <p>检测到的操作系统。安装逻辑与环境变量持久化会据此自动调整。</p>
          </div>
          <div class="setting-value">
            <span class="badge info">{{ osLabel }}</span>
            <span class="badge">v{{ info.app_version }}</span>
          </div>
        </div>
      </div>

      <div class="disclaimer">
        <h3>使用须知</h3>
        <p>
          ja-netfilter 是一个通用的 Java agent 框架，可用于调试、监控和扩展基于 JVM 的应用程序。
          请仅在您拥有合法授权的软件上使用；绕过付费授权可能违反软件服务条款及当地法律法规。
        </p>
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
}

.settings-grid {
  display: flex;
  flex-direction: column;
  gap: 14px;
  max-width: 880px;
}

.license-row {
  border-left: 2px solid var(--accent);
  margin-bottom: 18px;
}

.setting-row {
  background: var(--bg-1);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 16px;
  display: grid;
  grid-template-columns: 280px 1fr;
  gap: 16px;
}

.setting-label h3 {
  margin: 0 0 4px 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-0);
}
.setting-label p {
  margin: 0;
  font-size: 12px;
  color: var(--text-2);
  line-height: 1.5;
}
.setting-label code {
  font-family: var(--font-mono);
  font-size: 11.5px;
  background: var(--bg-2);
  padding: 1px 4px;
  border-radius: 3px;
  color: var(--accent);
}

.setting-value {
  display: flex;
  flex-direction: column;
  gap: 8px;
  align-items: flex-start;
}
.path {
  font-family: var(--font-mono);
  font-size: 12px;
  background: var(--bg-0);
  border: 1px solid var(--border-soft);
  border-radius: var(--radius);
  padding: 8px 10px;
  word-break: break-all;
  white-space: pre-wrap;
  color: var(--text-0);
  width: 100%;
}

.license-input {
  width: 100%;
  font-family: var(--font-mono);
  font-size: 13px;
}

.actions {
  display: flex;
  align-items: center;
  gap: 6px;
}

.hint {
  margin: 0;
  font-size: 11.5px;
  color: var(--text-2);
}
.hint code {
  font-family: var(--font-mono);
  color: var(--accent);
  background: var(--bg-2);
  padding: 1px 4px;
  border-radius: 3px;
}

.disclaimer {
  margin-top: 24px;
  padding: 16px;
  border-left: 2px solid var(--warn);
  background: rgba(251, 191, 36, 0.06);
  border-radius: var(--radius);
  max-width: 880px;
}
.disclaimer h3 {
  margin: 0 0 6px 0;
  font-size: 13px;
  color: var(--warn);
}
.disclaimer p {
  margin: 0;
  font-size: 12.5px;
  color: var(--text-1);
  line-height: 1.6;
}
</style>
