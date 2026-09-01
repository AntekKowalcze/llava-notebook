<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  numberOfContributions: number;
  date: string;
}>();

const opacity = computed(() => {
  const contributions = props.numberOfContributions;

  if (contributions === 0) return 0;
  return 0.15 + (Math.log(Math.min(contributions, 50) + 1) / Math.log(51)) * 0.85;
});

const titleString = computed(() => `${props.numberOfContributions} edition(s) • ${props.date}`);
</script>

<template>
  <div
    class="mr-0.5 h-4 w-4 rounded-[3px] border border-note-pumice/10"
    :class="props.numberOfContributions === 0 ? 'bg-note-graphite' : 'bg-note-graphite/40'"
    :title="titleString"
  >
    <div
      class="h-full w-full rounded-[3px] bg-note-paprika"
      :style="{ opacity }"
    />
  </div>
</template>

<style scoped></style>
