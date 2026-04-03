<script setup lang="ts">
const props = defineProps<{
  content: string;
  disabled?: boolean;
}>();
</script>

<template>
  <div
    class="mt-14 flex h-10 w-fit select-none items-center justify-center rounded-lg border-2 px-10 py-4 font-semibold tracking-wide backdrop-blur-sm transition-all duration-300 ease-out"
    :class="[
      !props.disabled
        ? `/* Hover Effects (Tylko gdy aktywny) */ /* Click Effects */ cursor-pointer border-note-ivory/20 bg-black/30 text-note-ivory hover:-translate-y-1 hover:border-note-paprika/60 hover:bg-note-garnet/20 hover:shadow-[0_0_20px_rgba(249,115,22,0.3)] active:translate-y-0 active:scale-95 active:border-note-paprika active:shadow-none`
        : `/* blocks clicking */ pointer-events-none cursor-not-allowed border-note-ivory/5 bg-black/10 text-note-ivory/20 opacity-50 grayscale`,
    ]"
    role="button"
    tabindex="0"
    @keydown.enter.prevent="$emit('click')"
    @keydown.space.prevent="$emit('click')"
  >
    {{ props.content }}
  </div>
</template>
