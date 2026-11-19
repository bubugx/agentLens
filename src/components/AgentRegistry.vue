<script setup lang="ts">
import { message } from "../lib/notifications";
import { computed, ref, watch } from "vue";
import { Unplug } from "lucide-vue-next";
import { t } from "../lib/i18n";
import type { AgentDefinition, Snapshot } from "../lib/types";
import { detachAgent } from "../lib/client";
import Modal from "./Modal.vue";
import AgentIcon from "./AgentIcon.vue";
const props = defineProps<{ data: Snapshot }>();
const definitions = computed(
  () =>
    props.data.agentDefinitions?.filter((d) => !d.builtin && d.enabled) || [],
);
const removing = ref<AgentDefinition | null>(null),
  busy = ref(false),
  error = ref("");
async function detach() {
  if (!removing.value) return;
  busy.value = true;
  error.value = "";
  try {
    await detachAgent(removing.value.id);
    removing.value = null;
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}
watch(error, (value) => {
  if (value) message(t(value.replace(/^Error: /, "")), "error");
});
</script>
<template>
  <section v-if="definitions.length" class="panel agent-registry">
    <div class="section-heading">
      <div>
        <h2>{{ t("Custom Agents") }}</h2>
        <p>{{ t("Your process rules and directory bindings.") }}</p>
      </div>
      <span class="count-pill">{{ definitions.length }}</span>
    </div>
    <div v-for="d in definitions" :key="d.id" class="agent-registry-row">
      <AgentIcon :id="d.id" />
      <div class="agent-registry-info">
        <strong>{{ d.name }}</strong><small
          >{{
            d.rule ? t("Saved recognition rule") : t("Current app session only")
          }}
          · {{ data.processes.filter((p) => p.agent === d.id).length }}
          {{ t("processes") }} ·
          {{ data.roots.filter((r) => r.agent === d.id).length }}
          {{ t("directories") }}</small
        ><code
          v-if="d.rule"
          :title="`${d.rule.executable} ${d.rule.entrypoint}`"
          >{{ d.rule.executable
          }}{{ d.rule.entrypoint ? ` · ${d.rule.entrypoint}` : "" }}</code
        >
      </div>
      <button
        class="button small"
        :aria-label="t('Detach {0}', d.name)"
        @click="
          removing = d;
          error = '';
        "
      >
        <Unplug :size="14" />{{ t("Detach") }}
      </button>
    </div>
  </section>
  <Modal
    v-if="removing"
    :title="t('Detach {0}?', removing.name)"
    @close="!busy && (removing = null)"
    ><p>
      {{
        t(
          "Recognition and directory monitoring for this Agent will stop. Its files and processes are kept, and historical labels remain available.",
        )
      }}
    </p>

    <template #footer
      ><button class="button" :disabled="busy" @click="removing = null">
        {{ t("Cancel") }}</button
      ><button class="button primary" :disabled="busy" @click="detach">
        {{ t("Confirm detach") }}
      </button></template
    ></Modal
  >
</template>
