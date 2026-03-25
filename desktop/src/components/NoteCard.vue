<script setup lang="ts">
import type { Note } from '../types/note';
const props = defineProps<{
  note: Note;
}>();
// function formDate(timestamp: number){
// const date = new Date(timestamp);
// return date.toLocaleDateString();
// }
</script>
<template>
  <div
    class="text-note-custard flex h-auto w-full flex-col rounded-lg bg-note-garnet p-5 backdrop-blur-lg hover:bg-red-900/70 active:bg-red-900/60"
  >
    <div class="flex h-6 w-full justify-between">
      <h3 class="text-lg font-black">{{ props.note.name }}</h3>

      <!-- <p class="font- text-sm ">l-id: {{props.note.local_id}}</p> -->
    </div>
    <h4 class="text-normal mt-2 flex h-8 w-full flex-wrap font-semibold">{{ props.note.title }}</h4>
    <p class="flex h-1/3 w-full flex-wrap text-base font-normal">{{ props.note.summary }}</p>
    <p class="text-note-custard/50 mx-auto flex h-5 w-full justify-start pt-3 text-xs font-normal">
      Owner: {{ props.note.owner_id }}
    </p>
  </div>
</template>
