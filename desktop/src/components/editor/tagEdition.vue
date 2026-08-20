<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { X, Plus, Tag, Check, Trash2 } from 'lucide-vue-next';
import { emit as emitTauri } from '@tauri-apps/api/event';
interface UiTag {
  tag_id: string;
  name: string;
  color: string;
}

const props = withDefaults(
  defineProps<{
    noteId: string;
    showBackdrop?: boolean;
  }>(),
  {
    showBackdrop: true,
  }
);

const emit = defineEmits<{
  close: [];
  changed: [];
}>();

const tags = ref<UiTag[]>([]);
const noteTags = ref<UiTag[]>([]);

const newTag = ref('');
const newTagColor = ref('#FACC15');

const loading = ref(false);
const creating = ref(false);
const error = ref<string | null>(null);

onMounted(async () => {
  await loadTags();
});

async function loadTags() {
  loading.value = true;
  error.value = null;

  try {
    tags.value = await invoke<UiTag[]>('get_all_tags');

    noteTags.value = await invoke<UiTag[]>('get_all_tags_for_note', {
      noteId: props.noteId,
    });
  } catch (err) {
    console.error('Failed to load tags:', err);
    error.value = 'Failed to load tags';
  } finally {
    loading.value = false;
  }
}

function hasTag(tag: UiTag) {
  return noteTags.value.some((item) => item.tag_id === tag.tag_id);
}

async function toggleTag(tag: UiTag) {
  error.value = null;

  try {
    if (hasTag(tag)) {
      await invoke('remove_tag_from_note', {
        noteId: props.noteId,
        tagName: tag.name,
      });

      noteTags.value = noteTags.value.filter((item) => item.tag_id !== tag.tag_id);
    } else {
      await invoke('add_tag_to_note', {
        noteId: props.noteId,
        tagName: tag.name,
        tagColor: tag.color,
      });

      noteTags.value.push(tag);
    }

    emit('changed');
    await emitTauri('tags_changed');
  } catch (err) {
    console.error('Failed to update tag:', err);
    error.value = 'Failed to update tag';
  }
}
async function createTag() {
  const name = newTag.value.trim();

  if (!name || creating.value) {
    return;
  }

  creating.value = true;
  error.value = null;

  try {
    await invoke('add_tag_to_note', {
      noteId: props.noteId,
      tagName: name,
      tagColor: newTagColor.value,
    });

    newTag.value = '';
    newTagColor.value = '#FACC15';

    await loadTags();

    emit('changed');
    await emitTauri('tags_changed');
  } catch (err) {
    console.error('Failed to create tag:', err);
    error.value = 'Failed to create tag';
  } finally {
    creating.value = false;
  }
}
async function removeTag(tag: UiTag) {
  error.value = null;

  try {
    await invoke('remove_tag', {
      tagId: tag.tag_id,
    });

    tags.value = tags.value.filter((item) => item.tag_id !== tag.tag_id);

    noteTags.value = noteTags.value.filter((item) => item.tag_id !== tag.tag_id);

    emit('changed');
    await emitTauri('tags_changed');
  } catch (err) {
    console.log(err);
    error.value = 'Failed to remove a tag';
  }
}
</script>

<template>
  <div
    class="fixed inset-0 z-50 flex items-center justify-center"
    :class="{
      'bg-black/55 backdrop-blur-sm': props.showBackdrop,
    }"
    @click.self="props.showBackdrop && emit('close')"
  >
    <div
      class="relative w-[420px] overflow-hidden rounded-xl border border-white/10 bg-note-graphite/95 shadow-2xl shadow-black/60 backdrop-blur-xl"
    >
      <!-- ambient glow -->
      <div
        class="pointer-events-none absolute -right-20 -top-20 h-40 w-40 rounded-full bg-note-glow/5 blur-3xl"
      />

      <div
        class="pointer-events-none absolute -bottom-20 -left-20 h-40 w-40 rounded-full bg-note-paprika/5 blur-3xl"
      />

      <!-- header -->
      <div class="relative flex items-center justify-between border-b border-white/10 px-5 py-4">
        <div class="flex items-center gap-3">
          <div
            class="flex h-8 w-8 items-center justify-center rounded-lg border border-note-glow/20 bg-note-glow/10 text-note-glow"
          >
            <Tag :size="15" />
          </div>

          <div>
            <h2 class="text-sm font-medium text-note-ivory">Edit tags</h2>

            <p class="mt-0.5 text-[11px] text-note-pumice/50">Organize this note with tags</p>
          </div>
        </div>

        <button
          class="rounded-md p-1.5 text-note-pumice/40 transition hover:bg-white/5 hover:text-note-ivory"
          @click="emit('close')"
        >
          <X :size="16" />
        </button>
      </div>

      <!-- content -->
      <div class="relative px-5 py-4">
        <!-- selected -->
        <div
          v-if="noteTags.length"
          class="mb-4"
        >
          <div class="mb-2 text-[10px] font-medium uppercase tracking-wider text-note-pumice/40">
            Selected
          </div>

          <div class="flex flex-wrap gap-1.5">
            <div
              v-for="tag in noteTags"
              :key="tag.tag_id"
              class="group flex items-center gap-1.5 rounded-md border px-2 py-1 text-[11px]"
              :style="{
                color: tag.color,
                borderColor: `${tag.color}35`,
                backgroundColor: `${tag.color}12`,
              }"
            >
              <span>#{{ tag.name }}</span>

              <button
                class="opacity-50 transition hover:opacity-100"
                :style="{ color: tag.color }"
                @click="toggleTag(tag)"
              >
                <X :size="11" />
              </button>
            </div>
          </div>
        </div>

        <!-- all tags -->
        <div>
          <div class="mb-2 text-[10px] font-medium uppercase tracking-wider text-note-pumice/40">
            Tags
          </div>

          <div
            v-if="loading"
            class="py-6 text-center text-xs text-note-pumice/40"
          >
            Loading tags...
          </div>

          <div
            v-else-if="!tags.length"
            class="rounded-lg border border-dashed border-white/10 py-6 text-center text-xs text-note-pumice/40"
          >
            No tags yet
          </div>

          <div
            v-else
            class="max-h-48 space-y-1 overflow-y-auto pr-1"
          >
            <div
              v-for="tag in tags"
              :key="tag.tag_id"
              class="group flex items-center gap-2 rounded-lg px-2.5 py-2 transition"
              :class="hasTag(tag) ? 'bg-white/5' : 'hover:bg-white/5'"
            >
              <button
                class="flex min-w-0 flex-1 items-center gap-2 text-left"
                @click="toggleTag(tag)"
              >
                <div
                  class="flex h-5 w-5 shrink-0 items-center justify-center rounded-md border transition"
                  :style="
                    hasTag(tag)
                      ? {
                          borderColor: `${tag.color}60`,
                          backgroundColor: `${tag.color}20`,
                          color: tag.color,
                        }
                      : {
                          borderColor: 'rgba(255,255,255,0.1)',
                          backgroundColor: 'rgba(255,255,255,0.03)',
                        }
                  "
                >
                  <Check
                    v-if="hasTag(tag)"
                    :size="12"
                  />
                </div>

                <span
                  class="truncate text-xs"
                  :style="{ color: tag.color }"
                >
                  #{{ tag.name }}
                </span>
              </button>

              <button
                class="rounded p-1 text-note-pumice/20 opacity-0 transition hover:bg-note-garnet/10 hover:text-note-garnet group-hover:opacity-100"
                @click="removeTag(tag)"
              >
                <Trash2 :size="12" />
              </button>
            </div>
          </div>
        </div>

        <!-- create -->
        <div class="mt-4 border-t border-white/5 pt-4">
          <div class="mb-2 text-[10px] font-medium uppercase tracking-wider text-note-pumice/40">
            Create tag
          </div>

          <div class="flex gap-2">
            <!-- name -->
            <div class="relative flex-1">
              <span
                class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-xs text-note-pumice/30"
              >
                #
              </span>

              <input
                v-model="newTag"
                type="text"
                placeholder="new tag"
                class="h-9 w-full rounded-lg border border-white/10 bg-black/20 pl-7 pr-3 text-xs text-note-ivory outline-none transition placeholder:text-note-pumice/25 focus:border-note-glow/30 focus:bg-black/30"
                @keydown.enter="createTag"
              />
            </div>

            <!-- color -->
            <label
              class="relative flex h-9 w-9 shrink-0 cursor-pointer items-center justify-center overflow-hidden rounded-lg border border-white/10 bg-black/20 transition hover:border-white/20"
              :style="{
                boxShadow: `0 0 14px ${newTagColor}20`,
              }"
            >
              <input
                v-model="newTagColor"
                type="color"
                class="absolute inset-0 h-full w-full cursor-pointer opacity-0"
              />

              <span
                class="h-4 w-4 rounded-full border border-white/20"
                :style="{ backgroundColor: newTagColor }"
              />
            </label>

            <!-- add -->
            <button
              class="flex h-9 items-center gap-1.5 rounded-lg border border-note-glow/20 bg-note-glow/10 px-3 text-xs text-note-glow transition hover:border-note-glow/30 hover:bg-note-glow/15 disabled:cursor-not-allowed disabled:opacity-40"
              :disabled="!newTag.trim() || creating"
              @click="createTag"
            >
              <Plus :size="13" />
              Add
            </button>
          </div>

          <!-- color preview -->
          <div class="mt-2 flex items-center gap-2">
            <span class="text-[10px] text-note-pumice/30">Color</span>

            <span
              class="text-[10px]"
              :style="{ color: newTagColor }"
            >
              {{ newTagColor }}
            </span>

            <span
              v-if="newTag"
              class="ml-auto text-[10px]"
              :style="{ color: newTagColor }"
            >
              #{{ newTag }}
            </span>
          </div>
        </div>

        <!-- error -->
        <div
          v-if="error"
          class="mt-3 rounded-lg border border-note-garnet/20 bg-note-garnet/10 px-3 py-2 text-[11px] text-note-garnet"
        >
          {{ error }}
        </div>
      </div>

      <!-- footer -->
      <div class="flex items-center justify-between border-t border-white/10 bg-black/10 px-5 py-3">
        <span class="text-[10px] text-note-pumice/30">{{ noteTags.length }} selected</span>

        <button
          class="rounded-lg border border-white/10 bg-white/5 px-3 py-1.5 text-xs text-note-pumice transition hover:bg-white/10 hover:text-note-ivory"
          @click="emit('close')"
        >
          Done
        </button>
      </div>
    </div>
  </div>
</template>
