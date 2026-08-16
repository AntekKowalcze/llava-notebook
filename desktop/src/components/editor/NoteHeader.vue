<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface UiTag {
  tag_id: string
  name: string
  color: string
}

const props = defineProps<{
  noteId: string
  noteName: string
}>()

const tags = ref<UiTag[]>([])
const loading = ref(false)

async function loadTags() {
  loading.value = true

  try {
    tags.value = await invoke<UiTag[]>(
      'get_all_tags_for_note',
      {
        noteId: props.noteId
      }
    )
  } catch (err) {
    console.error('Failed to load note tags:', err)
    tags.value = []
  } finally {
    loading.value = false
  }
}

onMounted(loadTags)

watch(
  () => props.noteId,
  () => {
    loadTags()
  }
)

defineExpose({
  loadTags
})
</script>

<template>
  <div
    class="flex h-11 shrink-0 items-center gap-3
           border-b border-white/5
            px-6
           backdrop-blur-md"
  >
    <!-- note name -->
    <div class="flex min-w-0 items-center">
      <span
        class="truncate text-lg font-normal  text-note-ivory"
      >
        {{ noteName }}
      </span>
    </div>

    <!-- divider -->
    <div class="h-4 w-px shrink-0 bg-white/10" />

    <!-- tags -->
    <div
      class="flex min-w-0 items-center gap-1.5 overflow-hidden "
    >
      <div
        v-for="tag in tags"
        :key="tag.tag_id"
        class="flex shrink-0 items-center rounded-md
               border px-2 py-[0.130rem]
               text-[10px] font-medium
               transition-all duration-150
               hover:brightness-110"
        :style="{
          color: tag.color,
          backgroundColor: `${tag.color}18`,
          borderColor: `${tag.color}35`
        }"
      >
        <span>#{{ tag.name }}</span>
      </div>

      <span
        v-if="!tags.length && !loading"
        class="text-[10px] text-note-pumice/25"
      >
        No tags
      </span>
    </div>
  </div>
</template>