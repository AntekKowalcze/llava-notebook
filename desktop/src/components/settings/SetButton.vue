<script setup lang="ts">
const props = defineProps<{
  id: string;
  content: string;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  (e: 'setting-changed', id: string, value: string): void;
}>();
</script>

<template>
  <button
    @click="() => emit('setting-changed', props.id, 'idle')"
    class="inline-flex select-none items-center justify-center rounded-lg border-2 px-4 py-1.5 text-xs font-medium tracking-wide transition-all duration-300 ease-linear"
    :class="
      props.disabled
        ? 'cursor-not-allowed border-note-ivory/10 bg-black/20 text-note-ivory/30 opacity-60'
        : 'border-note-ivory/20 bg-black/30 text-note-ivory/80 hover:border-note-paprika/50 hover:bg-black/40 hover:text-note-ivory active:scale-95'
    "
  >
    {{ props.content }}
  </button>
</template>
