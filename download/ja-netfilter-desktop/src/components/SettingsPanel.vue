<script setup lang="ts">
import { computed, onMounted } from "vue";

import { useSettingsStore } from "@/stores/settings";
import { useToast } from "@/stores/toast";

const settings = useSettingsStore();
const toast = useToast();

const info = computed(() => settings.info);

const osLabel = computed(() => {
  switch (info.value?.os) {
    case "windows":
      return "Windows";
    case "macos":
      return "macOS";
    case "linux":
      return "Linux";
    default:
      return "Unknown";
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
    const path = await import("@/api").then((m) => m.api.pickJarFile());
    if (path) toast.success(`Picked ${path}`);
  } catch (e: any) {
    toast.error(String(e));
  }
}

onMounted(() => settings.refresh());
</script>

<template>
  <section class="content-area">
    <header class="content-header">
      <div>
        <h2>Settings</h2>
        <p class="subtitle">Inspect the workspace, bundled jar and platform.</p>
      </div>
      <div class="toolbar">
        <button class="ghost" @click="settings.refresh()">Refresh</button>
      </div>
    </header>

    <div class="content-body">
      <div v-if="!info" class="empty">Loading…</div>
      <div v-else class="settings-grid">
        <div class="setting-row">
          <div class="setting-label">
            <h3>Workspace</h3>
            <p>Per-user directory that holds the writable copy of lib.jar, plugins, configs and vmoptions templates.</p>
          </div>
          <div class="setting-value">
            <code class="path">{{ info.workdir }}</code>
            <div class="actions">
              <button class="ghost" @click="reveal(info.workdir)">Reveal</button>
            </div>
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-label">
            <h3>ja-netfilter jar</h3>
            <p>The active javaagent jar loaded via <code>-javaagent:…=jetbrains</code> in each vmoptions file.</p>
          </div>
          <div class="setting-value">
            <code class="path">{{ info.jar_path }}</code>
            <div class="actions">
              <span class="badge" :class="info.jar_exists ? 'ok' : 'error'">
                {{ info.jar_exists ? "found" : "missing" }}
              </span>
              <button class="ghost" @click="reveal(info.jar_path)">Reveal</button>
              <button class="ghost" @click="pickJar">Pick custom…</button>
            </div>
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-label">
            <h3>Plugins directory</h3>
            <p>Where ja-netfilter looks for plugin jars at runtime.</p>
          </div>
          <div class="setting-value">
            <code class="path">{{ info.workdir }}/plugins</code>
            <div class="actions">
              <button class="ghost" @click="reveal(`${info.workdir}/plugins`)">Reveal</button>
            </div>
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-label">
            <h3>Config directory</h3>
            <p>Plugin rulesets read by dns / power / url at startup.</p>
          </div>
          <div class="setting-value">
            <code class="path">{{ info.config_dir }}</code>
            <div class="actions">
              <button class="ghost" @click="reveal(info.config_dir)">Reveal</button>
            </div>
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-label">
            <h3>vmoptions templates</h3>
            <p>Bundled per-product vmoptions files; used as a fallback when no per-user file exists.</p>
          </div>
          <div class="setting-value">
            <code class="path">{{ info.vmoptions_dir }}</code>
            <div class="actions">
              <button class="ghost" @click="reveal(info.vmoptions_dir)">Reveal</button>
            </div>
          </div>
        </div>

        <div class="setting-row">
          <div class="setting-label">
            <h3>Platform</h3>
            <p>Detected operating system. Install logic and env-var persistence adapt to this.</p>
          </div>
          <div class="setting-value">
            <span class="badge info">{{ osLabel }}</span>
            <span class="badge">v{{ info.app_version }}</span>
          </div>
        </div>
      </div>

      <div class="disclaimer">
        <h3>Responsible use</h3>
        <p>
          ja-netfilter is a generic Java agent framework. It can be used to debug, monitor and extend
          JVM-based applications. Only attach it to software you are licensed to run; circumventing
          paid license checks may violate the software's terms of service and local law.
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

.actions {
  display: flex;
  align-items: center;
  gap: 6px;
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
