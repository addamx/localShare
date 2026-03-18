import { createApp } from "vue";
import App from "./App.vue";
import router from "./router";
import "./styles/index.css";
import "./styles/theme.scss";

createApp(App).use(router).mount("#app");
