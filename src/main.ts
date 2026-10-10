import { createApp } from "vue";
import { createPinia } from "pinia";

import App from "@/App.vue";
import "@/styles/md3.css";

// 单页应用（MD3 顶部 Tab 导航），无路由依赖
const app = createApp(App);
app.use(createPinia());
app.mount("#app");
