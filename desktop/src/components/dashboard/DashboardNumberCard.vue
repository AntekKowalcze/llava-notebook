<template>
  <div>
    <div class="flex select-none flex-col items-start">
      <p
        class="mb-2 w-48 overflow-hidden text-ellipsis whitespace-nowrap text-[10px] font-medium uppercase tracking-wide text-note-pumice/70 sm:w-56 sm:text-xs md:w-72 md:text-sm md:tracking-widest"
      >
        {{ props.text }}
      </p>

      <div
        class="flex h-24 w-48 items-center justify-between rounded-xl border-2 border-note-pumice/40 bg-note-graphite/60 px-4 sm:h-28 sm:w-56 md:w-72 md:px-5"
      >
        <div class="flex w-12 shrink-0 items-center justify-center rounded-lg sm:w-14 md:w-16">
          <slot></slot>
        </div>

        <span class="truncate text-3xl font-semibold text-note-paprika sm:text-4xl md:text-5xl">
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
