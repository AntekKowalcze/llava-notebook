<script setup lang="ts">
import type { Note } from '../types/note'
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
        class="flex flex-col  bg-note-garnet backdrop-blur-lg  w-full h-auto p-5 rounded-lg text-note-custard hover:bg-red-900/70 active:bg-red-900/60 ">
        <div class="flex justify-between w-full h-6 ">
            <h3 class="font-black text-lg">{{ props.note.name }}</h3>

            <!-- <p class="font- text-sm ">l-id: {{props.note.local_id}}</p> -->
        </div>
        <h4 class="text-normal font-semibold flex flex-wrap w-full h-8 mt-2 ">{{ props.note.title }}</h4>
        <p class="text-base font-normal flex flex-wrap w-full h-1/3 ">{{ props.note.summary }}</p>
        <p class="text-note-custard/50 text-xs font-normal flex w-full justify-start mx-auto pt-3 h-5 ">Owner:
            {{ props.note.owner_id }}</p>
    </div>


</template>
