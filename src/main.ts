import { createApp } from "vue";
import App from "./App.vue";
import TrayPopover from "./components/TrayPopover.vue";
import "./styles.css";
createApp(new URLSearchParams(window.location.search).has("tray") ? TrayPopover : App).mount(
  "#root",
);
