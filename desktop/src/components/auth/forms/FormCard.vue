<script setup lang="ts">
const props = defineProps<{
  headerText: string;
  subText: string;
}>();
</script>
<template>
  <div
    class="group relative mx-auto my-auto flex h-fit min-h-[60vh] w-full max-w-xl shrink-0 flex-col items-center justify-center rounded-2xl border border-note-ivory/10 bg-note-graphite/40 p-[4vh] shadow-ember backdrop-blur-xl transition-all sm:min-h-[55vh] sm:p-10"
  >
    <div
      class="absolute inset-x-0 bottom-0 -z-10 h-3/4 rounded-b-2xl bg-gradient-to-t from-note-garnet/50 to-transparent blur-[50px]"
    ></div>
    <div class="absolute inset-0 -z-10 rounded-2xl bg-note-graphite/40"></div>

    <div class="relative z-10 flex h-full w-full flex-col items-center py-[2vh] sm:py-8">
      <!-- Header -->
      <div class="relative mb-[4vh] flex w-full flex-col items-center sm:mb-8">
        <h1
          class="select-none bg-gradient-to-br from-note-ivory to-note-glow bg-clip-text pb-2 text-[clamp(2rem,6vh,3rem)] font-bold text-transparent"
        >
          {{ props.headerText }}
        </h1>
        <p
          class="mt-1 select-none text-center text-xs uppercase tracking-widest text-note-ivory/50 sm:text-sm"
        >
          {{ props.subText }}
        </p>
      </div>

      <div
        class="flex min-h-[25vh] w-full flex-1 select-none flex-col items-center justify-center rounded border-note-ivory/20 p-[2vh] text-note-ivory/30 sm:min-h-[20vh] sm:p-5"
      >
        <slot></slot>
      </div>
    </div>
  </div>
</template>
