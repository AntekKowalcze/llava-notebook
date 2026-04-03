<script setup lang="ts">
const props = defineProps<{
  headerText: string;
  subText: string;
}>();
</script>
<template>
  <div
    class="group relative mx-auto mt-48 flex h-fit min-h-[625px] w-full max-w-xl flex-col items-center rounded-2xl border border-note-ivory/10 bg-note-graphite/40 p-10 shadow-ember backdrop-blur-xl"
  >
    <!-- light -->
    <div
      class="absolute inset-x-0 bottom-0 -z-10 h-3/4 min-h-[550px] rounded-b-2xl bg-gradient-to-t from-note-garnet/50 to-transparent blur-[50px]"
    ></div>
    <!-- Helper background if text not visible on light -->
    <div class="absolute inset-0 -z-10 rounded-2xl bg-note-graphite/40"></div>
    <!-- Card content -->
    <div class="relative z-10 flex min-h-[600px] w-full flex-col items-center py-16">
      <!-- Header -->
      <div class="relative mb-8 flex w-full flex-col items-center">
        <h1
          class="select-none bg-gradient-to-br from-note-ivory to-note-glow bg-clip-text pb-2 text-5xl font-bold text-transparent"
        >
          {{ props.headerText }}
        </h1>
        <p
          class="mt-1 select-none text-center text-sm uppercase tracking-widest text-note-ivory/50"
        >
          {{ props.subText }}
        </p>
      </div>
      <!-- Inputs -->
      <div
        class="flex h-fit w-full select-none flex-col items-center justify-center rounded border-note-ivory/20 p-5 text-note-ivory/30"
      >
        <slot></slot>
      </div>
    </div>
  </div>
</template>
