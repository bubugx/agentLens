<script setup lang="ts">
import { computed } from "vue";
import { Sparkles, ArrowRight } from "lucide-vue-next";
import { t } from "../lib/i18n";
import { assessmentCategory } from "../lib/processes";
import { bytes, compactPath, type Process, type Root } from "../lib/types";
import { projectFor } from "../lib/resources";
const props = defineProps<{
  processes: Process[];
  monitoring: boolean;
  roots: Root[];
}>();
const emit = defineEmits<{
  review: [];
  stopSuggested: [processes: Process[]];
}>();
const suggested = computed(() =>
  props.processes.filter((p) => assessmentCategory(p) === "suggested"),
);
const memory = computed(() =>
  suggested.value.reduce((s, p) => s + p.memory, 0),
);
const active = computed(
  () =>
    props.processes.filter((p) => assessmentCategory(p) === "active").length,
);
const review = computed(
  () =>
    props.processes.filter((p) => assessmentCategory(p) === "review").length,
);
const groups = computed(() => {
  const map = new Map<string, Process[]>();
  for (const p of suggested.value) {
    const key = projectFor(p, props.processes, props.roots).path;
    map.set(key, [...(map.get(key) || []), p]);
  }
  return [...map]
    .map(([path, processes]) => ({
      path,
      processes,
      memory: processes.reduce((s, p) => s + p.memory, 0),
    }))
    .sort((a, b) => b.memory - a.memory)
    .slice(0, 3);
});
</script>
<template>
  <section class="panel process-triage">
    <div class="section-heading">
      <div>
        <h2><Sparkles :size="16" />{{ t("Automatic process analysis") }}</h2>
        <p>
          {{
            t(
              "Group suspected leftovers by evidence and review the suggested batch once.",
            )
          }}
        </p>
      </div>
      <div class="inline-controls">
        <button class="button secondary small" @click="emit('review')">
          {{ t("View suggestions") }}</button><button
          class="button primary small"
          :disabled="!monitoring || !suggested.length"
          @click="emit('stopSuggested', [...suggested])"
        >
          {{ t("Review suggested batch ({0})", suggested.length)
          }}<ArrowRight :size="14" />
        </button>
      </div>
    </div>
    <div class="triage-statistics">
      <div>
        <span>{{ t("Suggested leftovers") }}</span><strong
          >{{ suggested.length
          }}<small
            >{{ t("processes") }} · RSS {{ bytes(memory) }}</small
          ></strong
        >
      </div>
      <div>
        <span>{{ t("Still working") }}</span><strong
          >{{ active }}<small>{{ t("processes") }}</small></strong
        >
      </div>
      <div>
        <span>{{ t("Needs further review") }}</span><strong
          >{{ review }}<small>{{ t("processes") }}</small></strong
        >
      </div>
    </div>
    <div v-if="groups.length" class="triage-projects">
      <button
        v-for="group in groups"
        :key="group.path"
        class="triage-project"
        :disabled="!monitoring"
        @click="emit('stopSuggested', group.processes)"
      >
        <span :title="group.path">{{
          group.path ? compactPath(group.path) : t("Unassigned project")
        }}</span><small
          >{{ group.processes.length }} {{ t("processes") }} ·
          {{ bytes(group.memory) }}</small
        ><ArrowRight :size="13" />
      </button>
    </div>
    <p class="triage-policy">
      {{
        !monitoring
          ? t("Resume monitoring to refresh process suggestions.")
          : t(
              "Suggestions require an observed exited parent, at least 5 minutes of continuous idle CPU and disk readings, and a recognized build or test task. Active tasks, services and launch ancestors are excluded. Activity is checked again before each stop.",
            )
      }}
    </p>
  </section>
</template>
