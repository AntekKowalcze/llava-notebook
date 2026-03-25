<script setup lang="ts">
import { computed } from 'vue';
const props = defineProps<{
  currentValue: string;
  id: string;
}>();
const emit = defineEmits<{
  (e: 'setting-changed', id: string, value: string): void;
}>();
const checked = computed(() => {
  if (props.currentValue == 'on') {
    return true;
  } else return false;
});
function changed() {
  emit('setting-changed', props.id, checked.value ? 'off' : 'on');
}
</script>

<template>
  <div class="relative inline-block h-5 w-11">
    <input
      :checked="checked"
      :id="props.id"
      type="checkbox"
      class="peer h-5 w-11 cursor-pointer appearance-none rounded-full border-2 border-note-ivory/20 bg-black/40"
      @change="changed"
    />
    <label
      :for="props.id"
      class="absolute left-0 top-0 h-5 w-5 cursor-pointer rounded-full bg-note-ivory shadow-sm [transition:transform_300ms_ease,background-color_300ms] peer-checked:translate-x-6 peer-checked:bg-note-paprika"
    ></label>
  </div>
</template>

<style lang="css" scoped></style>
