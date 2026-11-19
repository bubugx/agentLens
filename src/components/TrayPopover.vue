<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, shallowRef, ref } from "vue";
import {
  ArrowRight,
  ArrowUpRight,
  Cpu,
  HardDrive,
  MemoryStick,
  Pause,
  Play,
  Settings,
  Sparkles,
  X,
} from "lucide-vue-next";
import { invoke } from "@tauri-apps/api/core";
import * as client from "../lib/client";
import type { Snapshot } from "../lib/types";
import { bytes, delta } from "../lib/types";
import { t, setLocale, setTheme } from "../lib/i18n";
const props = defineProps<{ embedded?: boolean; data?: Snapshot }>();
const emit = defineEmits<{ close: []; navigate: [page: string] }>();
const keyboardNavigation = ref(false);
const current = shallowRef<Snapshot | null>(props.data || null);
let unlisten: (() => void) | undefined;
const data = computed(() => props.data || current.value);
const memory = computed(() => data.value?.agents.reduce((s, a) => s + a.memory, 0) || 0);
const cpu = computed(() => data.value?.agents.reduce((s, a) => s + a.cpu, 0) || 0);
const storage = computed(() => data.value?.agents.reduce((s, a) => s + a.storage, 0) || 0);
const cleanup = computed(() => data.value?.agents.reduce((s, a) => s + a.reclaimable, 0) || 0);
const active = computed(() => data.value?.agents.filter((a) => a.processes > 0).length || 0);
function receive(value: Snapshot) {
  current.value = value;
  setLocale(value.settings.language || "zh-CN");
  setTheme(value.settings.theme || "light", value.settings.customBase, value.settings.customAccent);
}
onMounted(async () => {
  document.addEventListener("keydown", dismissOnEscape);
  if (props.embedded) return;
  document.documentElement.classList.add("popover-host");
  unlisten = await client.subscribe(receive);
  receive(await client.snapshot());
});
onBeforeUnmount(() => {
  unlisten?.();
  document.removeEventListener("keydown", dismissOnEscape);
});
function dismissOnEscape(event: KeyboardEvent) {
  if (event.key === "Escape") close();
}
async function open(page: string) {
  if (props.embedded) {
    emit("navigate", page);
    emit("close");
  } else await invoke("open_main", { page });
}
async function close() {
  if (props.embedded) emit("close");
  else await invoke("hide_quicklook");
}
async function toggle() {
  if (data.value) await client.monitoring(!data.value.monitoring);
}
</script>
<template>
  <section
    v-if="data"
    class="tray-popover glass"
    role="dialog"
    :aria-label="t('Resource quick look')"
    @keydown.tab="keyboardNavigation = true"
    @pointermove="keyboardNavigation = false"
  >
    <div class="popover-heading">
      <img
        src="/icon.svg"
        alt=""
      />
      <div>
        <strong>AgentLens</strong>
        <span>
          <i :class="{ paused: !data.monitoring }" />
          {{ data.monitoring ? t("Watching your workspace") : t("Monitoring paused") }}
        </span>
      </div>
      <button
        class="icon-button popover-close"
        :class="{ 'keyboard-focused': keyboardNavigation }"
        :aria-label="t('Close dialog')"
        @click="close"
      >
        <X :size="15" />
      </button>
    </div>
    <div class="popover-resources">
      <div class="popover-resource">
        <span class="popover-label">
          <MemoryStick :size="15" />
          {{ t("Agent memory") }}
        </span>
        <strong>{{ bytes(memory) }}</strong>
        <span
          :class="['resource-change', { rising: (data.memoryDelta || 0) > 0 }]"
          :title="t('Since last sample')"
        >
          {{ delta(data.memoryDelta || 0) }}
        </span>
        <div class="popover-meter">
          <i
            :style="{
              width: `${Math.min(100, (memory / Math.max(1, data.totalMemory)) * 100)}%`,
            }"
          />
        </div>
      </div>
      <div class="popover-resource">
        <span class="popover-label">
          <Cpu :size="15" />
          {{ t("Agent CPU") }}
        </span>
        <strong>
          {{ cpu.toFixed(1) }}
          <em>%</em>
        </strong>
        <span
          :class="['resource-change', { rising: (data.cpuDelta || 0) > 0 }]"
          :title="t('Since last sample')"
        >
          {{ (data.cpuDelta || 0) >= 0 ? "+" : "−" }}{{ Math.abs(data.cpuDelta || 0).toFixed(1) }}
          {{ t("pp") }}
        </span>
        <div class="popover-meter cpu">
          <i :style="{ width: `${Math.min(100, cpu)}%` }" />
        </div>
      </div>
    </div>
    <div class="popover-details">
      <span>
        <HardDrive :size="14" />
        {{ t("Agent storage") }}
        <b>{{ bytes(storage) }}</b>
      </span>
      <span>
        <span class="popover-active-dot" />
        {{ t("Active agents") }}
        <b>
          {{ active }}
          <small>/ {{ data.processes.length }} {{ t("processes") }}</small>
        </b>
      </span>
    </div>
    <button
      class="popover-cleanup"
      @click="open('Cleanup')"
    >
      <span class="popover-cleanup-icon"><Sparkles :size="17" /></span>
      <span>
        <strong>{{ cleanup ? t("Cleanup available") : t("No cleanup candidates") }}</strong>
        <small v-if="cleanup">{{ bytes(cleanup) }}</small>
      </span>
      <ArrowRight :size="15" />
    </button>
    <div class="popover-footer">
      <button @click="open('Overview')">
        {{ t("Open AgentLens") }}
        <ArrowUpRight :size="12" />
      </button>
      <button
        class="icon-button"
        :aria-label="data.monitoring ? t('Pause monitoring') : t('Resume monitoring')"
        @click="toggle"
      >
        <Pause
          v-if="data.monitoring"
          :size="14"
        />
        <Play
          v-else
          :size="14"
        />
      </button>
      <button
        class="icon-button"
        :aria-label="t('Settings')"
        @click="open('Settings')"
      >
        <Settings :size="14" />
      </button>
    </div>
  </section>
</template>
