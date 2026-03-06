<script setup lang="ts">
const props = defineProps<{
    content: string
    disabled?: boolean
}>();
</script>

<template>
    <button class="
      inline-flex items-center justify-center
      px-4 py-1.5 rounded-lg
      text-xs font-medium tracking-wide
      border-2 transition-all duration-300 ease-linear
      select-none
    " :class="props.disabled
        ? 'border-note-ivory/10 bg-black/20 text-note-ivory/30 cursor-not-allowed opacity-60'
        : 'border-note-ivory/20 bg-black/30 text-note-ivory/80 hover:border-note-paprika hover:bg-black/40 hover:text-note-ivory active:scale-95'
        ">
        {{ props.content }}
    </button>
</template>
