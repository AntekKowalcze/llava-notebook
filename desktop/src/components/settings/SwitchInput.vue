<script setup lang="ts">
import { computed } from 'vue';
const props = defineProps<{
  currentValue: string,
  id: string
}>()
const emit = defineEmits<{
  (e: 'setting-changed', id: string, value: string): void
}>()
const checked = computed(() => {
  if (props.currentValue == "on") {
    return true
  } else return false

})
function changed() {
  emit('setting-changed', props.id, checked.value ? 'off' : 'on');
}

</script>


<template>
  <div class="relative inline-block w-11 h-5">
    <input :checked="checked" :id="props.id" type="checkbox"
      class="peer appearance-none w-11 h-5 bg-black/40 rounded-full border-2 border-note-ivory/20  cursor-pointer"
      @change="changed" />
    <label :for="props.id"
      class="absolute top-0 left-0 w-5 h-5 bg-note-ivory rounded-full  shadow-sm   peer-checked:translate-x-6 peer-checked:bg-note-paprika cursor-pointer [transition:transform_300ms_ease,background-color_300ms]">
    </label>
  </div>

</template>

<style lang="css" scoped></style>