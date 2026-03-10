<script setup lang="ts">
const props = defineProps<{
  state: boolean
}>()

</script>


<template>
<div class="relative inline-block w-11 h-5">
  <input :checked="state" id="switch-component" type="checkbox" class="peer appearance-none w-11 h-5 bg-black/40 rounded-full border-2 border-note-ivory/20  cursor-pointer" />
  <label for="switch-component" class="absolute top-0 left-0 w-5 h-5 bg-note-ivory rounded-full  shadow-sm   peer-checked:translate-x-6 peer-checked:bg-note-paprika cursor-pointer [transition:transform_300ms_ease,background-color_300ms]">
  </label>
</div>

</template>

<style lang="css" scoped>

</style>