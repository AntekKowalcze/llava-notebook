<script setup lang="ts">
import { ref } from 'vue';
const props = defineProps<{ checked: boolean; id: string }>();
const emit = defineEmits<{ (e: 'visibility-changed', id: string, value: boolean): void }>();
const checkboxState = ref<boolean>(props.checked);
function test() {
  checkboxState.value = !checkboxState.value;
  emit('visibility-changed', props.id, checkboxState.value);
}
</script>

<template>
  <div class="inline-flex items-center gap-4 text-note-ivory">
    <label class="relative flex cursor-pointer items-center">
      <input
        type="checkbox"
        :checked="props.checked"
        @change="test"
        class="peer h-5 w-5 cursor-pointer appearance-none rounded border-2 border-note-ivory/20 transition-all checked:border-note-paprika checked:bg-black/40"
      />
      <span
        class="pointer-events-none absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 transform text-white opacity-0 peer-checked:opacity-100"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          class="h-3.5 w-3.5"
          viewBox="0 0 20 20"
          fill="#F97316"
          stroke="#F97316"
          stroke-width="1"
        >
          <path
            fill-rule="evenodd"
            d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
            clip-rule="evenodd"
          ></path>
        </svg>
      </span>
    </label>
    {{ props.id }}
  </div>
</template>

<style lang="css" scoped></style>
