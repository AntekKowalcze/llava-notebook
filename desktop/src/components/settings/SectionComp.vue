<script setup lang="ts">
import { Section } from '../../types/settingTypes';
import SettingComp from './SettingComp.vue';

const props = defineProps<{
  section: Section;
}>();
const emit = defineEmits<{ (e: 'setting-changed', id: string, value: string): void }>();
</script>

<template>
  <div
    class="flex w-full flex-col"
    v-show="props.section.show"
  >
    <!-- Section Header -->
    <div class="flex items-center gap-3 px-6 py-3">
      <!-- Accent rail -->
      <div class="h-5 w-0.5 shrink-0 rounded-full bg-note-paprika/70"></div>

      <p
        class="select-none text-[11px] font-semibold uppercase tracking-[0.18em] text-note-pumice/50"
      >
        {{ props.section.sectionName }}
      </p>

      <!-- Trailing line -->
      <div class="h-px flex-1 bg-note-pumice/10"></div>
    </div>

    <!-- Settings list -->
    <div class="flex flex-col px-4 pb-2">
      <template v-if="props.section.sectionSettings.length > 0">
        <SettingComp
          v-for="setting in props.section.sectionSettings"
          :key="setting.id"
          :setting="setting"
          @setting-changed="(id, value) => emit('setting-changed', id, value)"
        />
      </template>

      <template v-if="props.section.subsections">
        <SectionComp
          v-for="sec in props.section.subsections"
          :key="sec.id"
          :section="sec"
          @setting-changed="(id, value) => emit('setting-changed', id, value)"
        />
      </template>
    </div>

    <!-- Bottom spacer / divider -->
    <div
      class="mx-6 mb-4 mt-2 h-px bg-gradient-to-r from-note-pumice/10 via-note-pumice/5 to-transparent"
    ></div>
  </div>
</template>
