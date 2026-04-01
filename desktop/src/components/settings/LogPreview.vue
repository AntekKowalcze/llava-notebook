<script setup lang="ts">
import Convert from 'ansi-to-html'
import { X } from 'lucide-vue-next';
const props = defineProps<{
    logContent: string,
}>();
const emit = defineEmits<{ (e: 'close-logs'): void }>()
function parseContent(): string {
    let convert = new Convert;
    let converted = convert.toHtml(props.logContent)
    return converted
}

</script>


<template>

    <div
        class="w-[60%] h-[60%] overflow-auto scrollbar-none bg-black text-note-pumice py-4 px-4  z-20 border-note-ivory/40 border rounded-lg">
        <div class="sticky top-3 z-30 flex justify-end">
            <X aria-label="close logs" class="text-note-pumice cursor-pointer p-2 h-10 w-10 "
                @click="() => { emit('close-logs') }"></X>
        </div>

        <div v-html="parseContent()" class="mt-2 log-content w-[90%] ml-6"></div>
    </div>
</template>

<style scoped>
.log-content {
    white-space: pre-wrap;
    line-height: 1.7;
    font-size: 0.95rem;
}

.log-content>* {
    margin-bottom: 0.5rem;
}
</style>
