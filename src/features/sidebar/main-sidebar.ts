import { createApp } from "vue";
import App from "./App.vue";
import { i18n } from "../../i18n";
import "../../style.css";
import "../chat/markdown/markdown-content.css";
import "./assets/sidebar-theme.css";
import { LUCIDE_CONTEXT } from "../../lucide-context";

createApp(App).use(i18n).provide(LUCIDE_CONTEXT, {}).mount("#app");
