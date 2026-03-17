<script setup lang="ts">
import { Section, Setting } from '../../types/settingTypes';
import SettingComp from './SettingComp.vue';

const props = defineProps<{
  section: Section,
}>()
const emit = defineEmits<{ (e: 'setting-changed', id: string, value: string): void }>()
</script>

<template>
  <div class="w-full flex flex-col">

    <!-- Section Header -->
    <div class="flex items-center gap-3 px-6 py-3">
      <!-- Accent rail -->
      <div class="w-0.5 h-5 rounded-full bg-note-paprika/70 shrink-0"></div>

      <p class="
        text-[11px] font-semibold tracking-[0.18em] uppercase
        text-note-pumice/50
        select-none
      ">
        {{ props.section.sectionName }}
      </p>

      <!-- Trailing line -->
      <div class="flex-1 h-px bg-note-pumice/10"></div>
    </div>

    <!-- Settings list -->
    <div class="flex flex-col px-4 pb-2">
      <SettingComp v-if="props.section.sectionSettings.length > 0" v-for="setting in props.section.sectionSettings"
        :key="setting.id" :setting="setting" @setting-changed="(id, value) => emit('setting-changed', id, value)">
      </SettingComp>
      <SectionComp v-if="props.section.subsections" v-for="sec in props.section.subsections" :key="sec.id"
        :section="sec" @setting-changed="(id, value) => emit('setting-changed', id, value)"></SectionComp>

    </div>

    <!-- Bottom spacer / divider -->
    <div class="mx-6 mt-2 mb-4 h-px bg-gradient-to-r from-note-pumice/10 via-note-pumice/5 to-transparent"></div>

  </div>
</template>