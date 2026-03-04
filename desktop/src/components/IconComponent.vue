<script setup lang="ts">
defineProps<{
    width: string,
    height: string
}>()
</script>
<template>
    <div>
        <img src="/icon.png" class=" drop-shadow-ember" :class="[width, height]" alt="" draggable="false">
    </div>
</template>



<style scoped></style>