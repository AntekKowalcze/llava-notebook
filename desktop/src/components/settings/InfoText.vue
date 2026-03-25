<script setup lang="ts">
interface Props {
  content: string;
}

const props = defineProps<Props>();
</script>

<template>
  <div
    class="max-w-sm rounded-lg border-2 border-note-ivory/20 bg-black/40 p-4 shadow-md backdrop-blur-sm transition duration-500 ease-in-out hover:border-note-paprika"
  >
    <p class="text-xs leading-relaxed text-note-ivory/70">{{ props.content }}</p>
  </div>
</template>
