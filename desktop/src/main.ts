import { createApp } from "vue";
import { createPinia } from "pinia";
import { router } from "./router/vueRouter"
import "./style.css";
import App from "./App.vue";
import '@fontsource/outfit/400.css';
import '@fontsource/outfit/700.css';
import Toast from "vue-toastification";
import "vue-toastification/dist/index.css";

const app = createApp(App);
app.use(createPinia());
app.use(router);

app.use(Toast as any, {
    transition: "Vue-Toastification__bounce",
    maxToasts: 20,
    newestOnTop: true,
    position: "bottom-center",
    toastClassName: "my-toast",
    icon: false,
});


app.mount("#app");
console.log("mounted")
