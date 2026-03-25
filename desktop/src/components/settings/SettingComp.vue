<script setup lang="ts">
import { computed } from 'vue';
import { Setting } from '../../types/settingTypes';
import InfoText from './InfoText.vue';
import SelectInput from './SelectInput.vue';
import SetButton from './SetButton.vue';
import SwitchInput from './SwitchInput.vue';
const props = defineProps<{
  setting: Setting;
}>();

const emit = defineEmits<{ (e: 'setting-changed', id: string, value: string): void }>();

console.log(props.setting.currentValue, props.setting.inputType);
</script>

<template>
  <div
    :id="props.setting.id"
    class="group relative flex items-center justify-between rounded-lg border border-transparent px-4 py-3 transition-all duration-300 ease-out hover:border-note-pumice/10 hover:bg-black/30"
    v-show="props.setting.show"
  >
    <!-- Left accent line — revealed on hover -->
    <div
      class="absolute left-0 top-1/2 h-0 w-0.5 -translate-y-1/2 rounded-full bg-note-paprika/0 transition-all duration-300 ease-out group-hover:h-5 group-hover:bg-note-paprika/60"
    ></div>

    <!-- Setting label + optional description -->
    <div class="flex min-w-0 flex-col pl-2">
      <p
        class="select-none truncate text-sm font-medium text-note-ivory/80 transition-colors duration-300 group-hover:text-note-ivory"
      >
        {{ props.setting.label }}
      </p>
      <p
        v-if="props.setting.description"
        class="mt-0.5 select-none text-[11px] leading-tight text-note-pumice/35"
      >
        {{ props.setting.description }}
      </p>
    </div>

    <!-- Input -->
    <div class="ml-6 shrink-0">
      <SelectInput
        v-if="props.setting.inputType === 'select'"
        :options="props.setting.options!"
        :id="props.setting.id"
        @setting-changed="(id, value) => emit('setting-changed', id, value)"
        :current-value="props.setting.currentValue"
      />
      <SetButton
        v-else-if="props.setting.inputType === 'button'"
        :content="props.setting.buttonLabel!"
      />
      <InfoText
        v-else-if="props.setting.inputType === 'info'"
        :content="props.setting.currentValue"
      />
      <SwitchInput
        v-else-if="props.setting.inputType === 'switch'"
        :current-value="props.setting.currentValue"
        :id="props.setting.id"
        @setting-changed="(id, value) => emit('setting-changed', id, value)"
      />
    </div>
  </div>
</template>
