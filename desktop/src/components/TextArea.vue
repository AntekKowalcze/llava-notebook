<script lang="ts" setup>
const props = defineProps<{ title: string; content: string }>();
</script>

<template>
  <div class="flex h-[90vh] w-3/4 flex-col">
    <button>add note</button>
    <textarea
      v-model="props.title"
      class="active:none h-[10%] bg-note-graphite text-3xl"
    ></textarea>
    <textarea
      v-model="props.content"
      class="h-full bg-note-graphite"
    ></textarea>
  </div>
</template>
