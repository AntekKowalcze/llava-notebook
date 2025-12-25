import { createApp } from "vue";
import { createPinia } from "pinia";
import { router } from "./router/vueRouter"
import "./style.css";
import App from "./App.vue";
import '@fontsource/outfit/400.css';
import '@fontsource/outfit/700.css';
const app = createApp(App);
app.use(createPinia());
app.use(router);

app.mount("#app");
console.log("mounted")
