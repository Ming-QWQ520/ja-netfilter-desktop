import { createApp } from "vue";
import { createPinia } from "pinia";
import { createRouter, createWebHashHistory } from "vue-router";

import App from "@/App.vue";
import "@/styles/main.css";

const routes = [
  { path: "/", name: "dashboard", component: () => import("@/components/Dashboard.vue") },
  { path: "/products", name: "products", component: () => import("@/components/Dashboard.vue") },
  { path: "/configs", name: "configs", component: () => import("@/components/ConfigEditor.vue") },
  { path: "/vmoptions", name: "vmoptions", component: () => import("@/components/VmoptionsViewer.vue") },
  { path: "/plugins", name: "plugins", component: () => import("@/components/PluginsPanel.vue") },
  { path: "/settings", name: "settings", component: () => import("@/components/SettingsPanel.vue") },
  { path: "/logs", name: "logs", component: () => import("@/components/LogConsole.vue") },
];

const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

const app = createApp(App);
app.use(createPinia());
app.use(router);
app.mount("#app");
