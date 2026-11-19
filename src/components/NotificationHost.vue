<script setup lang="ts">
import { CheckCircle2, Info, TriangleAlert, X } from "lucide-vue-next";
import {
  messages,
  dismissMessage,
  pauseMessage,
  resumeMessage,
} from "../lib/notifications";
import { t } from "../lib/i18n";
function leave(event: PointerEvent, id: number) {
  if (!(event.currentTarget as HTMLElement).contains(document.activeElement))
    resumeMessage(id);
}
function blur(event: FocusEvent, id: number) {
  const card = event.currentTarget as HTMLElement;
  if (
    !card.contains(event.relatedTarget as Node | null) &&
    !card.matches(":hover")
  )
    resumeMessage(id);
}
</script>
<template>
  <Teleport to="body"
    ><div class="notification-stack" :aria-label="t('Notifications')">
      <TransitionGroup name="message"
        ><div
          v-for="item in messages"
          :key="item.id"
          :class="['message-notification', item.tone]"
          :role="item.tone === 'error' ? 'alert' : 'status'"
          aria-atomic="true"
          @pointerenter="pauseMessage(item.id)"
          @pointerleave="leave($event, item.id)"
          @focusin="pauseMessage(item.id)"
          @focusout="blur($event, item.id)"
        >
          <component
            :is="
              item.tone === 'success'
                ? CheckCircle2
                : item.tone === 'info'
                  ? Info
                  : TriangleAlert
            "
            :size="18"
          />
          <div>
            <p>{{ item.text }}</p>
            <button
              v-if="item.action"
              class="text-button"
              @click="
                item.action();
                dismissMessage(item.id);
              "
            >
              {{ item.actionLabel }}
            </button>
          </div>
          <button
            class="icon-button"
            :aria-label="t('Dismiss notification')"
            @click="dismissMessage(item.id)"
          >
            <X :size="15" />
          </button></div
      ></TransitionGroup></div
  ></Teleport>
</template>
