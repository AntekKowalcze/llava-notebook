<script setup lang="ts">
import { computed } from 'vue'
import { Listbox, ListboxButton, ListboxOptions, ListboxOption } from '@headlessui/vue'

interface Props {
  options: string[]
  currentValue?: string
  id: string
}

const props = defineProps<Props>()
const emit = defineEmits<{
  (e: 'setting-changed', id: string, value: string): void,
}>()
//model value is choosen and 
const selected = computed({
  get: () => props.currentValue ?? props.options[0] ?? '',
  set: (value) => {
    emit('setting-changed', props.id, value);
  }
})

</script>

<template>
  <Listbox v-model="selected" as="div" class="relative">
    <ListboxButton class="
        inline-flex items-center justify-center
        px-4 py-1.5 rounded-lg
        text-xs font-medium tracking-wide
        border-2 transition-all duration-300 ease-linear
        select-none bg-black/30 border-note-ivory/20 text-note-ivory/80
        hover:border-note-paprika/50 hover:bg-black/40 hover:text-note-ivory
        active:scale-95 data-[headlessui-state~=open]:ring-2 ring-note-paprika/50
      ">
      {{ selected || 'Select...' }}
    </ListboxButton>

    <ListboxOptions class="
        absolute z-50 mt-1 rounded-lg border-2 border-note-ivory/20
        bg-note-graphite/95 backdrop-blur-sm shadow-xl py-1.5 max-h-60 overflow-auto
        focus:outline-none
      ">
      <ListboxOption v-for="option in props.options" :key="option" :value="option" as="template" v-slot="{ selected }">
        <li class="
    relative flex cursor-pointer select-none items-center
    px-3 py-2 text-xs font-medium transition-all duration-300 ease-linear
    max-w-40 data-[headlessui-state~=active]:bg-note-graphite/60 
    data-[headlessui-state~=active]:text-note-paprika/80
    data-[headlessui-state~=selected]:text-note-paprika 
    data-[headlessui-state~=selected]:font-semibold
  ">
          <span class="block truncate flex-1  text-note-paprika">{{ option }}</span>
          <span v-show="selected" class="ml-2 text-note-paprika">
            <svg class="h-3.5 w-3.5" fill="currentColor" viewBox="0 0 20 20">
              <path fill-rule="evenodd"
                d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" />
            </svg>
          </span>
        </li>
      </ListboxOption>
    </ListboxOptions>
  </Listbox>
</template>
