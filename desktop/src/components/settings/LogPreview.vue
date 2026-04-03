<script setup lang="ts">
import Convert from 'ansi-to-html';
import { X } from 'lucide-vue-next';
const props = defineProps<{
  logContent: string;
}>();
const emit = defineEmits<{ (e: 'close-logs'): void }>();
function parseContent(): string {
  let convert = new Convert();
  let converted = convert.toHtml(props.logContent);
  return converted;
}
</script>

<template>
  <div
    class="scrollbar-none z-20 h-[60%] w-[60%] overflow-auto rounded-lg border border-note-ivory/40 bg-black px-4 py-4 text-note-pumice"
  >
    <div class="sticky top-3 z-30 flex justify-end">
      <X
        aria-label="close logs"
        class="h-10 w-10 cursor-pointer p-2 text-note-pumice"
        @click="
          () => {
            emit('close-logs');
          }
        "
      ></X>
    </div>

    <div
      v-html="parseContent()"
      class="log-content ml-6 mt-2 w-[90%]"
    ></div>
  </div>
</template>

<style scoped>
.log-content {
  white-space: pre-wrap;
  line-height: 1.7;
  font-size: 0.95rem;
}

.log-content > * {
  margin-bottom: 0.5rem;
}
</style>
