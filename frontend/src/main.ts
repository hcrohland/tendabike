import "./app.css";
import { mount } from "svelte";
import App from "./App.svelte";
import { setLocale, extractLocaleFromNavigator } from "./lib/paraglide/runtime";

const isDev = import.meta.env.DEV;

if (!isDev) {
  const detected = extractLocaleFromNavigator();
  if (detected) setLocale(detected);
}

// @ts-ignore
const app = mount(App, { target: document.getElementById("app") });

export default app;
