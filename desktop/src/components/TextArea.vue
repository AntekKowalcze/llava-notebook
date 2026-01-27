<script lang="ts" setup>
const props = defineProps<{title: string, content: string}>()
</script>

<template>
    <div class="flex flex-col w-3/4 h-[90vh]">
        <button>add note</button>
        <textarea v-model="props.title" class="h-[10%] bg-note-graphite text-3xl active:none"></textarea>
        <textarea v-model="props.content" class="h-full bg-note-graphite"></textarea>
    </div>

</template>