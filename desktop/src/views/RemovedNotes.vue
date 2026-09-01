<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { ArrowBigLeftDash, Trash2, RotateCcw } from 'lucide-vue-next';
import { invoke } from '@tauri-apps/api/core';
import { useRouter } from 'vue-router';
import { useToast } from 'vue-toastification';

import ScreenDeviderHorizontal from '../components/dashboard/ScreenDeviderHorizontal.vue';
import { useAuthStore } from '../stores/auth';

interface UiTag {
  tag_id: string;
  name: string;
  color: string;
}

interface RemovedNote {
  local_id: string;
  title: string;
  removed_at: number;
  tags: UiTag[];
}

const router = useRouter();
const toast = useToast();
const authStore = useAuthStore();

const notes = ref<RemovedNote[]>([]);
const loading = ref(true);
const currentTime = ref(Date.now());

const DELETION_PERIOD = 30 * 24 * 60 * 60 * 1000;

function getVisibleTags(note: RemovedNote): UiTag[] {
  return note.tags.filter((tag) => tag.name.toLowerCase() !== 'favourites');
}

function getRemainingTime(removedAt: number, now: number): string {
  const expiresAt = removedAt + DELETION_PERIOD;
  const remaining = Math.max(0, expiresAt - now);

  const days = Math.floor(remaining / 86_400_000);
  const hours = Math.floor((remaining % 86_400_000) / 3_600_000);
  const minutes = Math.floor((remaining % 3_600_000) / 60_000);

  if (remaining <= 0) {
    return 'Deleting soon';
  }

  if (days > 0) {
    return `${days} day${days === 1 ? '' : 's'} left`;
  }

  if (hours > 0) {
    return `${hours} hour${hours === 1 ? '' : 's'} left`;
  }

  return `${minutes} minute${minutes === 1 ? '' : 's'} left`;
}

function getRemainingColor(removedAt: number, now: number): string {
  const expiresAt = removedAt + DELETION_PERIOD;
  const remaining = Math.max(0, expiresAt - now);

  const days = remaining / 86_400_000;

  if (days <= 3) {
    return 'text-note-garnet';
  }

  if (days <= 7) {
    return 'text-note-glow';
  }

  return 'text-note-pumice/60';
}

async function loadRemovedNotes() {
  loading.value = true;

  try {
    const loadedNotes = await invoke<Omit<RemovedNote, 'tags'>[]>('get_all_removed_notes_data', {
      userId: authStore.loggedInUserId,
    });

    const result: RemovedNote[] = [];

    for (const note of loadedNotes) {
      try {
        const tags = await invoke<UiTag[]>('get_all_tags_for_note', {
          noteId: note.local_id,
        });

        result.push({
          ...note,
          tags,
        });
      } catch (err) {
        console.error(`Failed to load tags for removed note ${note.local_id}:`, err);

        result.push({
          ...note,
          tags: [],
        });
      }
    }

    notes.value = result;
  } catch (err) {
    console.error(err);
    toast.error('Failed to load removed notes');
  } finally {
    loading.value = false;
  }
}

function goBack() {
  router.back();
}

onMounted(async () => {
  await loadRemovedNotes();
});

async function hardDeleteNote(note: RemovedNote) {
  try {
    await invoke<void>('hard_delete_note', { noteId: note.local_id });
    await loadRemovedNotes();
  } catch (err) {
    toast.error('Failed to delete note');
  }
}

async function restoreNote(note: RemovedNote) {
  try {
    await invoke<void>('restore_note', { noteId: note.local_id });
    await loadRemovedNotes();
  } catch (err) {
    toast.error('Failed to restore note');
  }
}
</script>

<template>
  <ArrowBigLeftDash
    class="fixed bottom-[7%] left-[2%] z-10 cursor-pointer text-note-paprika/80 transition-transform duration-200 hover:scale-95 active:scale-90"
    @click.stop="goBack"
  />

  <div class="relative h-full min-h-0 overflow-y-auto overflow-x-hidden">
    <header class="shrink-0 px-[10%] pb-5 pt-8">
      <h1 class="text-4xl font-semibold tracking-tight text-note-ivory lg:text-5xl">
        Removed
        <span class="text-note-paprika">notes</span>
      </h1>

      <p class="mt-2 text-sm text-note-pumice/50">
        Your notes will be permanently deleted 30 days after removal.
      </p>
    </header>

    <ScreenDeviderHorizontal class="shrink-0" />

    <main class="px-[10%] pb-12 pt-6">
      <div
        v-if="loading"
        class="flex h-64 items-center justify-center text-sm text-note-pumice/40"
      >
        Loading removed notes...
      </div>

      <div
        v-else-if="notes.length === 0"
        class="flex h-64 items-center justify-center"
      >
        <div class="text-center">
          <Trash2 class="mx-auto mb-3 h-8 w-8 text-note-pumice/20" />

          <p class="text-sm font-medium text-note-ivory/70">No removed notes</p>

          <p class="mt-1 text-xs text-note-pumice/40">Notes you remove will appear here.</p>
        </div>
      </div>

      <div
        v-else
        class="grid grid-cols-1 gap-3 xl:grid-cols-2"
      >
        <article
          v-for="note in notes"
          :key="note.local_id"
          class="relative flex h-32 min-w-0 flex-col overflow-visible rounded-xl border border-note-pumice/15 bg-black/60 p-4 shadow-lg shadow-black/20 transition-colors duration-200 hover:border-note-pumice/30 hover:bg-black/70"
        >
          <!-- Top section -->
          <div class="flex min-h-0 flex-1 items-start gap-3">
            <div class="min-w-0 flex-1">
              <!-- Title -->
              <h3 class="min-w-0 truncate text-base font-semibold text-note-ivory">
                {{ note.title }}
              </h3>

              <!-- Tags -->
              <div
                v-if="getVisibleTags(note).length > 0"
                class="mt-2 flex min-w-0 flex-wrap gap-1.5"
              >
                <span
                  v-for="tag in getVisibleTags(note).slice(0, 3)"
                  :key="tag.tag_id"
                  class="rounded-md border px-2 py-0.5 text-[10px] font-medium"
                  :style="{
                    backgroundColor: `${tag.color}18`,
                    borderColor: `${tag.color}55`,
                    color: tag.color,
                  }"
                >
                  {{ tag.name }}
                </span>

                <span
                  v-if="getVisibleTags(note).length > 3"
                  class="rounded-md border border-note-pumice/15 bg-black/20 px-2 py-0.5 text-[10px] font-medium text-note-pumice/50"
                >
                  +{{ getVisibleTags(note).length - 3 }}
                </span>
              </div>
            </div>
          </div>

          <!-- Bottom section -->
          <div class="flex shrink-0 items-center justify-between gap-4">
            <!-- Time remaining -->
            <div
              class="flex items-center gap-2 text-[11px]"
              :class="getRemainingColor(note.removed_at, currentTime)"
            >
              <span class="whitespace-nowrap">
                {{ getRemainingTime(note.removed_at, currentTime) }}
              </span>
            </div>

            <!-- Actions -->
            <div class="flex items-center gap-1">
              <!-- Restore -->
              <button
                type="button"
                class="group flex items-center gap-1.5 rounded-md px-2 py-1 text-[11px] font-medium text-note-pumice/60 transition-colors duration-200 hover:bg-note-pumice/10 hover:text-note-pumice"
                title="Restore this note"
                @click.stop="restoreNote(note)"
              >
                <RotateCcw
                  class="h-3.5 w-3.5 transition-transform duration-200 group-hover:-rotate-45"
                />
                <span>Restore</span>
              </button>

              <!-- Hard delete -->
              <button
                type="button"
                class="group flex items-center gap-1.5 rounded-md px-2 py-1 text-[11px] font-medium text-note-garnet/70 transition-colors duration-200 hover:bg-note-garnet/10 hover:text-note-garnet"
                title="Permanently delete this note"
                @click.stop="hardDeleteNote(note)"
              >
                <Trash2
                  class="h-3.5 w-3.5 transition-transform duration-200 group-hover:scale-105"
                />
                <span>Delete permanently</span>
              </button>
            </div>
          </div>
        </article>
      </div>
    </main>
  </div>
</template>

<!-- Create here removed notes component, which will look similar to allNotes view, i want it to use the same collor palette
 
          graphite: '#0F0F10', // Background
          garnet: '#9F1239', //Red accent
          paprika: '#F97316', // Orange accent
          glow: '#FACC15', // Yellow accent
          pumice: '#E7E5E4', // Grey
          ivory: '#FFFBEB', //  Light ideal for text

    i want on the top title "Removed notes" and under i want to have some text like 
    your notes will be deleted 30 after removing them 
    on the card i want to have title, and time left to delete note i also want to see tags
    

-->
