<template>
  <div>
    <div class="flex select-none flex-col items-start">
      <!-- label over the card -->
      <p class="mb-2 text-sm font-medium uppercase tracking-widest text-note-pumice/70">
        {{ props.text }}
      </p>

      <div
        class="flex h-32 w-64 items-center justify-between rounded-xl border-2 border-note-pumice/40 bg-note-graphite/60 px-5"
      >
        <div class="h flex w-16 items-center justify-center rounded-lg">
          <slot></slot>
        </div>

        <span class="text-5xl font-semibold text-note-paprika">
          {{ props.count }}
        </span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
const props = defineProps<{
  count: number;
  text: string;
}>();
</script>

<style scoped></style>
