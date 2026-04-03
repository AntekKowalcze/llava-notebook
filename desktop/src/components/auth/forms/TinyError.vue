<script setup lang="ts">
const props = defineProps<{ errorContent: string }>();
</script>

<template>
  <div class="mt-5 w-[55%] text-sm text-note-paprika">
    {{ props.errorContent }}
  </div>
</template>
