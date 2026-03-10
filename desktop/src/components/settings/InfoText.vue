<script setup lang="ts">
interface Props {
  content: string
}

const props = defineProps<Props>()
</script>

<template>
  <div class="
    p-4 rounded-lg border-2 border-note-ivory/20
    bg-black/40 backdrop-blur-sm shadow-md
    max-w-sm
    transition duration-500 ease-in-out
    hover:border-note-paprika
  ">
    
    <p class="text-xs text-note-ivory/70 leading-relaxed">{{ props.content }}</p>
  </div>
</template>
