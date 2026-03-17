<script setup lang="ts">
import { computed } from 'vue';
import { Setting } from '../../types/settingTypes';
import InfoText from './InfoText.vue';
import SelectInput from './SelectInput.vue';
import SetButton from './SetButton.vue';
import SwitchInput from './SwitchInput.vue';
const props = defineProps<{
  setting: Setting,

}>()

const emit = defineEmits<{ (e: 'setting-changed', id: string, value: string): void }>()

console.log(props.setting.currentValue, props.setting.inputType)

</script>

<template>
  <div :id="props.setting.id" class="
    group
    relative flex items-center justify-between
    px-4 py-3 rounded-lg
    border border-transparent
    transition-all duration-300 ease-out
    hover:bg-black/30
    hover:border-note-pumice/10
  ">

    <!-- Left accent line — revealed on hover -->
    <div class="
      absolute left-0 top-1/2 -translate-y-1/2
      w-0.5 rounded-full
      bg-note-paprika/0 group-hover:bg-note-paprika/60
      transition-all duration-300 ease-out
      h-0 group-hover:h-5
    "></div>

    <!-- Setting label + optional description -->
    <div class="flex flex-col min-w-0 pl-2">
      <p class="
        text-sm font-medium
        text-note-ivory/80 group-hover:text-note-ivory
        transition-colors duration-300
        select-none truncate
      ">
        {{ props.setting.label }}
      </p>
      <p v-if="props.setting.description" class="text-[11px] text-note-pumice/35 mt-0.5 leading-tight select-none">
        {{ props.setting.description }}
      </p>
    </div>

    <!-- Input -->
    <div class="shrink-0 ml-6">
      <SelectInput v-if="props.setting.inputType === 'select'" :options="props.setting.options!" :id="props.setting.id"
        @setting-changed="(id, value) => emit('setting-changed', id, value)"
        :current-value="props.setting.currentValue" />
      <SetButton v-else-if="props.setting.inputType === 'button'" :content="props.setting.buttonLabel!" />
      <InfoText v-else-if="props.setting.inputType === 'info'" :content="props.setting.currentValue" />
      <SwitchInput v-else-if="props.setting.inputType === 'switch'" :current-value="props.setting.currentValue"
        :id="props.setting.id" @setting-changed="(id, value) => emit('setting-changed', id, value)" />
    </div>

  </div>
</template>