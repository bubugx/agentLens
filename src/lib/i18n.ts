import { ref } from "vue";
import zh from "./locales/zh-CN.json";
export type Language = "zh-CN" | "en-US";
export type Theme = "light" | "dark" | "custom";
function stored(key: string, fallback: string) {
  try {
    if (typeof window === "undefined") return fallback;
    return window.localStorage.getItem(key) || fallback;
  } catch {
    return fallback;
  }
}
export const locale = ref<Language>(
  stored("agentlens.locale", "zh-CN") === "en-US" ? "en-US" : "zh-CN",
);
export const theme = ref<Theme>(
  ["light", "dark", "custom"].includes(stored("agentlens.theme", "light"))
    ? (stored("agentlens.theme", "light") as Theme)
    : "light",
);
export const customBase = ref<"light" | "dark">(
  stored("agentlens.customBase", "light") === "dark" ? "dark" : "light",
);
export const customAccent = ref(
  /^#[0-9a-f]{6}$/i.test(stored("agentlens.customAccent", "#167c83"))
    ? stored("agentlens.customAccent", "#167c83")
    : "#167c83",
);
export function setLocale(value: Language) {
  locale.value = value;
  try {
    if (typeof window !== "undefined")
      window.localStorage.setItem("agentlens.locale", value);
  } catch {}
  if (typeof document !== "undefined") document.documentElement.lang = value;
}
export function setTheme(
  value: Theme,
  base = customBase.value,
  accent = customAccent.value,
) {
  const normalized = ["light", "dark", "custom"].includes(value)
    ? value
    : "light";
  theme.value = normalized;
  customBase.value = base;
  customAccent.value = /^#[0-9a-f]{6}$/i.test(accent) ? accent : "#167c83";
  try {
    if (typeof window !== "undefined") {
      window.localStorage.setItem("agentlens.theme", normalized);
      window.localStorage.setItem("agentlens.customBase", base);
      window.localStorage.setItem("agentlens.customAccent", customAccent.value);
    }
  } catch {}
  if (typeof document !== "undefined") {
    const root = document.documentElement;
    root.dataset.theme = normalized;
    root.dataset.colorScheme = normalized === "custom" ? base : normalized;
    const rgb = customAccent.value
      .slice(1)
      .match(/../g)!
      .map((v) => parseInt(v, 16));
    const luminance = rgb
      .map((v) => v / 255)
      .map((v) => (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4))
      .reduce((sum, v, i) => sum + v * [0.2126, 0.7152, 0.0722][i], 0);
    root.style.setProperty("--custom-accent", customAccent.value);
    root.style.setProperty(
      "--custom-on-accent",
      luminance > 0.179 ? "#12202d" : "#ffffff",
    );
    root.style.colorScheme = root.dataset.colorScheme;
  }
}
const dictionary = zh as Record<string, string>;
export function t(message: string, ...values: (string | number)[]): string {
  const result =
    locale.value === "zh-CN" ? dictionary[message] || message : message;
  return result.replace(/\{(\d+)\}/g, (_, index) =>
    String(values[Number(index)] ?? ""),
  );
}
export const pageLabel = (page: string) => t(page);
export function categoryLabel(category: string) {
  return t(
    (
      {
        sessions: "Sessions",
        cache: "Cache",
        session_cache: "Session cache",
        temporary: "Temporary",
        logs: "Logs",
        settings: "Settings",
        project: "Project",
        other: "Other",
      } as Record<string, string>
    )[category] || category,
  );
}
export function rootLabel(label: string) {
  return locale.value === "zh-CN"
    ? label.replace(/ storage$/, " 存储目录")
    : label;
}
setLocale(locale.value);
setTheme(theme.value);
