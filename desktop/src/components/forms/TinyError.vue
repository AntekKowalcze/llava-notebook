<script setup lang="ts">
const props = defineProps<{ errorContent: string }>()

</script>

<template>
    <div class="text-note-paprika text-sm mt-5 w-[55%] ">
        {{ props.errorContent }}
    </div>
</template>