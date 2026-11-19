import { shallowRef } from "vue";
export interface Message {
  id: number;
  key: string;
  text: string;
  tone: "success" | "warning" | "error" | "info";
  action?: () => void;
  actionLabel?: string;
}
export const messages = shallowRef<Message[]>([]);
let next = 0;
const timers = new Map<
  number,
  {
    handle?: ReturnType<typeof setTimeout>;
    remaining: number;
    started: number;
  }
>();
export function dismissMessage(id: number) {
  clearTimeout(timers.get(id)?.handle);
  timers.delete(id);
  messages.value = messages.value.filter((m) => m.id !== id);
}
export function pauseMessage(id: number) {
  const timer = timers.get(id);
  if (!timer?.handle) return;
  clearTimeout(timer.handle);
  timer.remaining = Math.max(0, timer.remaining - (Date.now() - timer.started));
  timer.handle = undefined;
}
export function resumeMessage(id: number) {
  const timer = timers.get(id);
  if (!timer || timer.handle || timer.remaining <= 0) return;
  timer.started = Date.now();
  timer.handle = setTimeout(() => dismissMessage(id), timer.remaining);
}
export function message(
  text: string,
  tone: Message["tone"] = "info",
  options: {
    key?: string;
    action?: () => void;
    actionLabel?: string;
    duration?: number;
  } = {},
) {
  const key = options.key || `${tone}:${text}`;
  if (messages.value.some((m) => m.key === key)) return;
  if (messages.value.length >= 4) dismissMessage(messages.value[0].id);
  const id = ++next;
  messages.value = [
    ...messages.value,
    {
      id,
      key,
      text,
      tone,
      action: options.action,
      actionLabel: options.actionLabel,
    },
  ];
  const duration = options.duration ?? (tone === "error" || tone === "warning" ? 10000 : 7000);
  if (duration > 0) {
    timers.set(id, { remaining: duration, started: Date.now() });
    resumeMessage(id);
  }
}
