<script setup lang="ts">
const props = withDefaults(defineProps<{ whyDisabled: string; id: string; checked: boolean }>(), {
  checked: false,
});
</script>
<template>
  <div class="relative inline-block h-5 w-11">
    <input
      :checked="checked"
      :id="props.id"
      type="checkbox"
      disabled
      :title="props.whyDisabled"
      class="peer h-5 w-11 cursor-not-allowed appearance-none rounded-full border-2 border-white/10 bg-black/20"
    />
    <label
      :for="props.id"
      class="absolute left-0 top-0 h-5 w-5 cursor-not-allowed rounded-full bg-white/30 shadow-sm peer-checked:translate-x-6 peer-checked:bg-white/40"
      :title="props.whyDisabled"
    ></label>
  </div>
</template>
