<script setup lang="ts">
import {
  ArrowBigLeftDash,
  ArrowRight,
  Cloud,
  CloudOff,
  Ellipsis,
  Lock,
  LockOpen,
  Search,
  Star,
  RefreshCw,
  Trash2,
  Tags,
  Pencil,
  Check,
  X,
} from 'lucide-vue-next';

import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue';

import { Listbox, ListboxButton, ListboxOptions, ListboxOption } from '@headlessui/vue';

import { invoke } from '@tauri-apps/api/core';
import { useRouter } from 'vue-router';
import { useToast } from 'vue-toastification';

import ScreenDeviderHorizontal from '../components/dashboard/ScreenDeviderHorizontal.vue';
import TagEdition from '../components/editor/tagEdition.vue';

import { useAuthStore } from '../stores/auth';
import { useCurrentNoteStore } from '../stores/currentNoteStore';
import { useLayoutStore } from '../stores/layoutStore';
import { emit } from '@tauri-apps/api/event';
interface UiTag {
  tag_id: string;
  name: string;
  color: string;
}

interface Note {
  local_id: string;
  title: string;
  updated_at: number;
  encrypted: boolean;
  sync_state: string;
}

interface NoteWithTags extends Note {
  tags: UiTag[];
}

interface NoteGroup {
  key: string;
  label: string;
  notes: NoteWithTags[];
}

interface MenuPosition {
  top: number;
  left: number;
}

const router = useRouter();
const toast = useToast();
const authStore = useAuthStore();
const currentNoteStore = useCurrentNoteStore();
const layoutStore = useLayoutStore();

const notes = ref<NoteWithTags[]>([]);
const loading = ref(true);

const searchText = ref('');
const selectedDate = ref('all');
const selectedStatus = ref('all');

const openedMenuId = ref<string | null>(null);
const menuPosition = ref<MenuPosition | null>(null);

const editingTitleId = ref<string | null>(null);
const editingTitle = ref('');
const titleSaving = ref(false);

const currentTime = ref(Date.now());

const timeAgoInterval = window.setInterval(() => {
  currentTime.value = Date.now();
}, 60_000);

const statusOptions = [
  {
    value: 'all',
    label: 'All notes',
  },
  {
    value: 'encrypted',
    label: 'Encrypted',
  },
  {
    value: 'unencrypted',
    label: 'Not encrypted',
  },
  {
    value: 'synced',
    label: 'Synced',
  },
  {
    value: 'local',
    label: 'Local only',
  },
];

function formatTimeAgo(timestamp: number, now: number): string {
  const milliseconds = Math.max(0, now - timestamp);

  if (milliseconds < 60_000) {
    return 'just now';
  }

  if (milliseconds < 3_600_000) {
    const minutes = Math.floor(milliseconds / 60_000);

    return `${minutes} minute${minutes === 1 ? '' : 's'} ago`;
  }

  if (milliseconds < 86_400_000) {
    const hours = Math.floor(milliseconds / 3_600_000);

    return `${hours} hour${hours === 1 ? '' : 's'} ago`;
  }

  if (milliseconds < 2_592_000_000) {
    const days = Math.floor(milliseconds / 86_400_000);

    return `${days} day${days === 1 ? '' : 's'} ago`;
  }

  if (milliseconds < 31_536_000_000) {
    const weeks = Math.floor(milliseconds / 604_800_000);

    return `${weeks} week${weeks === 1 ? '' : 's'} ago`;
  }

  const months = Math.floor(milliseconds / 2_592_000_000);

  return `${months} month${months === 1 ? '' : 's'} ago`;
}

function formatDateKey(timestamp: number): string {
  const date = new Date(timestamp);

  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');

  return `${year}-${month}-${day}`;
}

function getDateLabel(timestamp: number): string {
  const date = new Date(timestamp);
  const now = new Date();

  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());

  const yesterday = new Date(today);
  yesterday.setDate(yesterday.getDate() - 1);

  const target = new Date(date.getFullYear(), date.getMonth(), date.getDate());

  if (target.getTime() === today.getTime()) {
    return 'Today';
  }

  if (target.getTime() === yesterday.getTime()) {
    return 'Yesterday';
  }

  return new Intl.DateTimeFormat('en-GB', {
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  }).format(date);
}

function getDateOptions(): Array<{
  value: string;
  label: string;
}> {
  const uniqueDates = new Set(notes.value.map((note) => formatDateKey(note.updated_at)));

  return [
    {
      value: 'all',
      label: 'All dates',
    },
    ...Array.from(uniqueDates)
      .sort()
      .reverse()
      .map((date) => ({
        value: date,
        label: getDateLabel(new Date(`${date}T00:00:00`).getTime()),
      })),
  ];
}

function getSyncLabel(syncState: string): string {
  switch (syncState) {
    case 'LocalOnly':
      return 'Local only';

    case 'PendingUpload':
      return 'Syncing';

    case 'PendingDownload':
      return 'Syncing';

    case 'Synced':
      return 'Synced';

    case 'Conflict':
      return 'Conflict';

    case 'Error':
      return 'Sync error';

    case 'PendingDeleted':
      return 'Deleting';
    case 'WaitingForTombstone':
      return 'Syncing deletation';
    default:
      return 'Unknown';
  }
}

function getSyncIcon(syncState: string) {
  switch (syncState) {
    case 'LocalOnly':
      return CloudOff;

    case 'PendingUpload':
    case 'PendingDownload':
      return RefreshCw;

    default:
      return Cloud;
  }
}

function getSyncColor(syncState: string): string {
  switch (syncState) {
    case 'LocalOnly':
      return 'text-note-pumice/50';

    case 'PendingUpload':
    case 'PendingDownload':
      return 'text-note-glow';

    case 'Synced':
      return 'text-note-pumice/80';

    case 'Conflict':
    case 'Error':
    case 'WaitingForTombstone':
      return 'text-note-garnet';

    default:
      return 'text-note-paprika';
  }
}

function isFavourite(note: NoteWithTags): boolean {
  return note.tags.some((tag) => tag.name.toLowerCase() === 'favourites');
}

function getVisibleTags(note: NoteWithTags): UiTag[] {
  return note.tags.filter((tag) => tag.name.toLowerCase() !== 'favourites');
}

const filteredNotes = computed(() => {
  const query = searchText.value.trim().toLowerCase();

  return notes.value.filter((note) => {
    const matchesSearch = query.length === 0 || note.title.toLowerCase().includes(query);

    const matchesDate =
      selectedDate.value === 'all' || formatDateKey(note.updated_at) === selectedDate.value;

    const matchesStatus =
      selectedStatus.value === 'all' ||
      (selectedStatus.value === 'encrypted' && note.encrypted) ||
      (selectedStatus.value === 'unencrypted' && !note.encrypted) ||
      (selectedStatus.value === 'synced' && note.sync_state === 'Synced') ||
      (selectedStatus.value === 'local' && note.sync_state === 'LocalOnly');

    return matchesSearch && matchesDate && matchesStatus;
  });
});

const groupedNotes = computed<NoteGroup[]>(() => {
  const groups = new Map<string, NoteWithTags[]>();

  const sorted = [...filteredNotes.value].sort((a, b) => b.updated_at - a.updated_at);

  for (const note of sorted) {
    const key = formatDateKey(note.updated_at);

    if (!groups.has(key)) {
      groups.set(key, []);
    }

    groups.get(key)!.push(note);
  }

  return Array.from(groups.entries()).map(([key, groupNotes]) => ({
    key,
    label: getDateLabel(groupNotes[0].updated_at),
    notes: groupNotes,
  }));
});

const dateOptions = computed(() => getDateOptions());

const dateFilterValue = computed({
  get: () => selectedDate.value,
  set: (value) => {
    selectedDate.value = value;
  },
});

const statusFilterValue = computed({
  get: () => selectedStatus.value,
  set: (value) => {
    selectedStatus.value = value;
  },
});

function closeMenu() {
  openedMenuId.value = null;
  menuPosition.value = null;
}

async function toggleMenu(noteId: string, event: MouseEvent) {
  if (editingTitleId.value !== null) {
    cancelTitleEdit();
  }

  if (openedMenuId.value === noteId) {
    closeMenu();
    return;
  }

  const target = event.currentTarget;

  if (!(target instanceof HTMLElement)) {
    return;
  }

  const rect = target.getBoundingClientRect();

  const menuWidth = 208;
  const viewportPadding = 12;

  let left = rect.right - menuWidth;

  if (left < viewportPadding) {
    left = viewportPadding;
  }

  if (left + menuWidth > window.innerWidth - viewportPadding) {
    left = window.innerWidth - menuWidth - viewportPadding;
  }

  menuPosition.value = {
    top: rect.bottom + 6,
    left,
  };

  openedMenuId.value = noteId;

  await nextTick();
}

function openNote(noteId: string) {
  closeMenu();

  router.push(`/main/editor/${noteId}`);
}

function startTitleEdit(note: NoteWithTags) {
  closeMenu();

  editingTitleId.value = note.local_id;

  editingTitle.value = note.title;
}

function cancelTitleEdit() {
  editingTitleId.value = null;
  editingTitle.value = '';
}

async function submitTitle(note: NoteWithTags) {
  const title = editingTitle.value.trim();

  if (!title) {
    toast.warning('Title cannot be empty');
    return;
  }

  if (title === note.title) {
    cancelTitleEdit();
    return;
  }

  titleSaving.value = true;

  try {
    await invoke<void>('change_note_title', {
      noteId: note.local_id,
      title,
    });

    note.title = title;
    note.updated_at = Date.now();

    cancelTitleEdit();

    toast.success('Title changed successfully');
  } catch (err) {
    console.error(err);

    toast.error('Failed to change title');
  } finally {
    titleSaving.value = false;
  }
}

async function manageTags(note: NoteWithTags) {
  closeMenu();

  try {
    const noteObject = await invoke<Note>('get_note_object', {
      noteId: note.local_id,
    });

    currentNoteStore.currentNote = noteObject as any;

    layoutStore.openTagEditor();
  } catch (err) {
    console.error(err);

    toast.error('Failed to open tag manager');
  }
}

async function toggleFavourite(note: NoteWithTags) {
  closeMenu();

  try {
    const favourite = isFavourite(note);

    if (favourite) {
      await invoke<void>('remove_tag_from_note', {
        noteId: note.local_id,
        tagName: 'favourites',
      });

      note.tags = note.tags.filter((tag) => tag.name.toLowerCase() !== 'favourites');

      toast.success('Removed from favourites');
    } else {
      await invoke<void>('add_tag_to_note', {
        noteId: note.local_id,
        tagName: 'favourites',
        tagColor: '#FACC15',
      });

      note.tags.push({
        tag_id: 'favourites',
        name: 'favourites',
        color: '#FACC15',
      });

      toast.success('Added to favourites');
    }
  } catch (err) {
    console.error(err);

    toast.error('Failed to change favourite state');
  }
}
function openEncryptionSettings(note: NoteWithTags) {
  closeMenu();
  toast.info('To change encryption method click plus button');

  router.push(`/main/editor/${note.local_id}`);
}

async function toggleSync(note: NoteWithTags) {
  closeMenu();

  const nextValue = note.sync_state === 'LocalOnly' ? 'on' : 'off';

  try {
    await invoke<void>('toggle_note_sync', {
      noteId: note.local_id,
      value: nextValue,
    });

    note.sync_state = nextValue === 'off' ? 'LocalOnly' : 'PendingUpload';

    note.updated_at = Date.now();

    toast.success(
      nextValue === 'on' ? 'Note synchronization enabled' : 'Note synchronization disabled'
    );
  } catch (err) {
    console.error(err);

    toast.error('Failed to change note synchronization');
  }
}

async function deleteNote(note: NoteWithTags) {
  closeMenu();

  try {
    await invoke<void>('remove_note', {
      noteId: note.local_id,
    });

    notes.value = notes.value.filter((current) => current.local_id !== note.local_id);
    await emit("reload-left-panel")
    toast.success('Note deleted');
  } catch (err) {
    console.error(err);

    toast.error('Failed to delete note');
  }
}

async function loadNotes() {
  loading.value = true;

  try {
    const loadedNotes = await invoke<Note[]>('get_all_notes_data', {
      userId: authStore.loggedInUserId,
    });

    const result: NoteWithTags[] = [];

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
        console.error(`Failed to load tags for note ${note.local_id}:`, err);

        result.push({
          ...note,
          tags: [],
        });
      }
    }

    notes.value = result;
  } catch (err) {
    console.error(err);

    toast.error('Failed to load notes');
  } finally {
    loading.value = false;
  }
}

function closeTagEditor() {
  layoutStore.closeTagEditor();
  loadNotes();
}

function redirect() {
  console.log('redirecing');
  router.replace({ name: 'create' });
}

onMounted(() => {
  loadNotes();
});

onUnmounted(() => {
  window.clearInterval(timeAgoInterval);
});
</script>

<template>
  <ArrowBigLeftDash
    class="fixed bottom-[7%] left-[2%] z-10 cursor-pointer text-note-paprika/80 transition-transform duration-200 hover:scale-95 active:scale-90"
    @click.stop="redirect"
  />
  <div class="relative h-full min-h-0 overflow-y-auto overflow-x-hidden">
    <Teleport to="body">
      <div
        v-if="openedMenuId && menuPosition"
        class="fixed z-[10000] w-52 overflow-hidden rounded-lg border border-note-pumice/15 bg-note-graphite p-1 shadow-2xl shadow-black/70"
        :style="{
          top: `${menuPosition.top}px`,
          left: `${menuPosition.left}px`,
        }"
        @click.stop
      >
        <template
          v-for="note in notes"
          :key="note.local_id"
        >
          <template v-if="note.local_id === openedMenuId">
            <button
              type="button"
              class="flex w-full items-center gap-2.5 rounded-md px-2.5 py-2 text-left text-xs text-note-pumice/75 transition-colors hover:bg-white/[0.04] hover:text-note-ivory"
              @click="toggleFavourite(note)"
            >
              <Star class="h-4 w-4 text-note-glow" />

              <span>
                {{ isFavourite(note) ? 'Remove favourite' : 'Add to favourites' }}
              </span>
            </button>

            <button
              type="button"
              class="flex w-full items-center gap-2.5 rounded-md px-2.5 py-2 text-left text-xs text-note-pumice/75 transition-colors hover:bg-white/[0.04] hover:text-note-ivory"
              @click="startTitleEdit(note)"
            >
              <Pencil class="h-4 w-4 text-note-pumice/70" />

              <span>Rename note</span>
            </button>

            <button
              type="button"
              class="flex w-full items-center gap-2.5 rounded-md px-2.5 py-2 text-left text-xs text-note-pumice/75 transition-colors hover:bg-white/[0.04] hover:text-note-ivory"
              @click="manageTags(note)"
            >
              <Tags class="h-4 w-4 text-note-pumice/70" />

              <span>Manage tags</span>
            </button>

            <button
              type="button"
              class="flex w-full items-center justify-between gap-2.5 rounded-md px-2.5 py-2 text-left text-xs text-note-pumice/75 transition-colors hover:bg-white/[0.04] hover:text-note-ivory"
              @click="openEncryptionSettings(note)"
            >
              <div class="flex items-center gap-2.5">
                <component
                  :is="note.encrypted ? Lock : LockOpen"
                  class="h-4 w-4 text-note-glow"
                />

                <span>Encryption</span>
              </div>

              <span class="text-note-paprika">Open</span>
            </button>
            <button
              type="button"
              class="flex w-full items-center gap-2.5 rounded-md px-2.5 py-2 text-left text-xs text-note-pumice/75 transition-colors hover:bg-white/[0.04] hover:text-note-ivory"
              @click="toggleSync(note)"
            >
              <component
                :is="note.sync_state === 'LocalOnly' ? CloudOff : Cloud"
                class="h-4 w-4 text-note-pumice/70"
              />

              <span>
                {{ note.sync_state === 'LocalOnly' ? 'Enable sync' : 'Disable sync' }}
              </span>
            </button>

            <div class="my-1 border-t border-note-pumice/10" />

            <button
              type="button"
              class="flex w-full items-center gap-2.5 rounded-md px-2.5 py-2 text-left text-xs text-note-garnet transition-colors hover:bg-note-garnet/10"
              @click="deleteNote(note)"
            >
              <Trash2 class="h-4 w-4" />

              <span>Delete note</span>
            </button>
          </template>
        </template>
      </div>
    </Teleport>

    <TagEdition
      v-if="layoutStore.isTagEditorOpen && currentNoteStore.currentNote"
      class="absolute left-1/2 top-1/2 z-[1000] -translate-x-1/2 -translate-y-1/2"
      :note-id="currentNoteStore.currentNote.local_id"
      @close="closeTagEditor"
      :show-backdrop="false"
    />

    <header class="shrink-0 px-[10%] pb-5 pt-8">
      <div class="flex items-end justify-between gap-8">
        <div>
          <h1 class="text-4xl font-semibold tracking-tight text-note-ivory lg:text-5xl">
            All
            <span class="text-note-paprika">notes</span>
          </h1>

          <p class="mt-2 text-sm text-note-pumice/50">Your notes, organized by last update.</p>
        </div>

        <div class="shrink-0 text-right">
          <p class="text-xs uppercase tracking-[0.2em] text-note-pumice/40">
            {{ filteredNotes.length }}
            {{ filteredNotes.length === 1 ? 'note' : 'notes' }}
          </p>
        </div>
      </div>
    </header>

    <ScreenDeviderHorizontal class="shrink-0" />

    <div class="shrink-0 px-[10%] py-4">
      <div class="flex flex-col gap-3 lg:flex-row lg:items-center">
        <div
          class="flex h-10 min-w-0 flex-1 items-center rounded-lg border border-note-pumice/20 bg-black/40 px-3 transition-colors focus-within:border-note-paprika/60 focus-within:bg-black/60"
        >
          <Search class="mr-2 h-4 w-4 shrink-0 text-note-paprika" />

          <input
            v-model="searchText"
            type="text"
            placeholder="Search by note title..."
            class="min-w-0 flex-1 bg-transparent text-sm text-note-ivory outline-none placeholder:text-note-pumice/40"
          />
        </div>

        <div class="flex gap-2">
          <Listbox
            v-model="dateFilterValue"
            as="div"
            class="relative"
          >
            <ListboxButton
              class="inline-flex h-10 min-w-36 select-none items-center justify-between gap-3 rounded-lg border border-note-pumice/20 bg-black/30 px-4 text-xs font-medium tracking-wide text-note-ivory/80 ring-note-paprika/50 transition-all duration-300 ease-linear hover:border-note-paprika/50 hover:bg-black/40 hover:text-note-ivory data-[headlessui-state~=open]:ring-2"
            >
              <span class="truncate">
                {{
                  dateOptions.find((option) => option.value === selectedDate)?.label ?? 'All dates'
                }}
              </span>

              <ArrowRight class="h-3.5 w-3.5 shrink-0 rotate-90 text-note-pumice/50" />
            </ListboxButton>

            <ListboxOptions
              class="absolute left-0 z-[500] mt-1 max-h-60 min-w-full overflow-auto rounded-lg border border-note-pumice/20 bg-note-graphite/95 py-1.5 shadow-xl focus:outline-none"
            >
              <ListboxOption
                v-for="option in dateOptions"
                :key="option.value"
                :value="option.value"
                as="template"
                v-slot="{ selected, active }"
              >
                <li
                  class="relative flex cursor-pointer select-none items-center px-3 py-2 text-xs font-medium transition-colors"
                  :class="{
                    'bg-note-graphite/80 text-note-paprika': active,
                    'text-note-ivory': !active,
                    'font-semibold': selected,
                  }"
                >
                  <span class="block flex-1 truncate">
                    {{ option.label }}
                  </span>

                  <span
                    v-show="selected"
                    class="ml-2 text-note-paprika"
                  >
                    ✓
                  </span>
                </li>
              </ListboxOption>
            </ListboxOptions>
          </Listbox>

          <Listbox
            v-model="statusFilterValue"
            as="div"
            class="relative"
          >
            <ListboxButton
              class="inline-flex h-10 min-w-32 select-none items-center justify-between gap-3 rounded-lg border border-note-pumice/20 bg-black/30 px-4 text-xs font-medium tracking-wide text-note-ivory/80 ring-note-paprika/50 transition-all duration-300 ease-linear hover:border-note-paprika/50 hover:bg-black/40 hover:text-note-ivory data-[headlessui-state~=open]:ring-2"
            >
              <span class="truncate">
                {{
                  statusOptions.find((option) => option.value === selectedStatus)?.label ??
                  'All notes'
                }}
              </span>

              <ArrowRight class="h-3.5 w-3.5 shrink-0 rotate-90 text-note-pumice/50" />
            </ListboxButton>

            <ListboxOptions
              class="absolute right-0 z-[500] mt-1 max-h-60 min-w-full overflow-auto rounded-lg border border-note-pumice/20 bg-note-graphite/95 py-1.5 shadow-xl focus:outline-none"
            >
              <ListboxOption
                v-for="option in statusOptions"
                :key="option.value"
                :value="option.value"
                as="template"
                v-slot="{ selected, active }"
              >
                <li
                  class="relative flex cursor-pointer select-none items-center px-3 py-2 text-xs font-medium transition-colors"
                  :class="{
                    'bg-note-graphite/80 text-note-paprika': active,
                    'text-note-ivory': !active,
                    'font-semibold': selected,
                  }"
                >
                  <span class="block flex-1 truncate">
                    {{ option.label }}
                  </span>

                  <span
                    v-show="selected"
                    class="ml-2 text-note-paprika"
                  >
                    ✓
                  </span>
                </li>
              </ListboxOption>
            </ListboxOptions>
          </Listbox>
        </div>
      </div>
    </div>

    <main class="px-[10%] pb-12 pt-2">
      <div
        v-if="loading"
        class="flex h-64 items-center justify-center text-sm text-note-pumice/40"
      >
        Loading notes...
      </div>

      <div
        v-else-if="groupedNotes.length === 0"
        class="flex h-64 items-center justify-center"
      >
        <div class="text-center">
          <Search class="mx-auto mb-3 h-8 w-8 text-note-pumice/20" />

          <p class="text-sm font-medium text-note-ivory/70">No notes found</p>

          <p class="mt-1 text-xs text-note-pumice/40">Try changing your search or filters.</p>
        </div>
      </div>

      <div
        v-else
        class="flex flex-col gap-8"
      >
        <section
          v-for="group in groupedNotes"
          :key="group.key"
        >
          <div class="mb-3">
            <div class="mb-2 flex items-center justify-between">
              <h2 class="text-xs font-semibold uppercase tracking-[0.2em] text-note-pumice/70">
                {{ group.label }}
              </h2>

              <span class="text-[11px] text-note-pumice/30">
                {{ group.notes.length }}
                {{ group.notes.length === 1 ? 'note' : 'notes' }}
              </span>
            </div>

            <ScreenDeviderHorizontal />
          </div>

          <div class="grid grid-cols-1 gap-3 xl:grid-cols-2">
            <article
              v-for="note in group.notes"
              :key="note.local_id"
              class="relative flex h-32 min-w-0 flex-col overflow-visible rounded-xl border border-note-pumice/15 bg-black/60 p-4 shadow-lg shadow-black/20 transition-colors duration-200 hover:border-note-pumice/30 hover:bg-black/70"
            >
              <div class="flex min-h-0 flex-1 items-start gap-3">
                <div class="min-w-0 flex-1">
                  <div
                    v-if="editingTitleId !== note.local_id"
                    class="flex min-w-0 items-center gap-2"
                  >
                    <h3 class="min-w-0 truncate text-base font-semibold text-note-ivory">
                      {{ note.title }}
                    </h3>
                  </div>

                  <div
                    v-else
                    class="flex min-w-0 items-center gap-2"
                  >
                    <input
                      v-model="editingTitle"
                      type="text"
                      autofocus
                      class="min-w-0 flex-1 rounded-md border border-note-paprika/40 bg-black/50 px-2 py-1 text-sm text-note-ivory outline-none focus:border-note-paprika/70"
                      :disabled="titleSaving"
                      @keydown.enter="submitTitle(note)"
                      @keydown.esc="cancelTitleEdit()"
                    />

                    <button
                      type="button"
                      class="shrink-0 text-note-glow hover:text-note-ivory disabled:opacity-40"
                      :disabled="titleSaving"
                      @click="submitTitle(note)"
                    >
                      <Check class="h-4 w-4" />
                    </button>

                    <button
                      type="button"
                      class="shrink-0 text-note-garnet hover:text-note-ivory disabled:opacity-40"
                      :disabled="titleSaving"
                      @click="cancelTitleEdit"
                    >
                      <X class="h-4 w-4" />
                    </button>
                  </div>

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

                <div class="flex shrink-0 items-center gap-2">
                  <button
                    v-if="isFavourite(note)"
                    type="button"
                    class="text-note-glow transition-transform hover:scale-110"
                    title="Favourite"
                    @click.stop="toggleFavourite(note)"
                  >
                    <Star class="h-4 w-4 fill-note-glow" />
                  </button>

                  <span
                    v-if="note.encrypted"
                    class="text-note-glow"
                    title="Encrypted"
                  >
                    <Lock class="h-4 w-4" />
                  </span>

                  <span
                    v-else
                    class="text-note-pumice/30"
                    title="Not encrypted"
                  >
                    <LockOpen class="h-4 w-4" />
                  </span>

                  <button
                    type="button"
                    class="flex h-8 w-8 items-center justify-center rounded-md text-note-pumice/40 transition-colors hover:bg-black/50 hover:text-note-ivory"
                    title="More options"
                    @click.stop="toggleMenu(note.local_id, $event)"
                  >
                    <Ellipsis class="h-4 w-4" />
                  </button>
                </div>
              </div>

              <div class="flex shrink-0 items-end justify-between gap-4">
                <div class="flex min-w-0 items-center gap-3">
                  <div
                    class="flex items-center gap-1.5 text-[11px]"
                    :class="getSyncColor(note.sync_state)"
                  >
                    <component
                      :is="getSyncIcon(note.sync_state)"
                      class="h-3.5 w-3.5"
                    />

                    <span class="whitespace-nowrap">
                      {{ getSyncLabel(note.sync_state) }}
                    </span>
                  </div>

                  <span class="text-note-pumice/20">•</span>

                  <span class="truncate text-[11px] text-note-pumice/45">
                    Updated
                    {{ formatTimeAgo(note.updated_at, currentTime) }}
                  </span>
                </div>

                <button
                  type="button"
                  class="group/open flex shrink-0 items-center gap-1.5 text-xs font-medium text-note-paprika transition-colors hover:text-note-glow"
                  @click="openNote(note.local_id)"
                >
                  Open

                  <ArrowRight
                    class="h-3.5 w-3.5 transition-transform group-hover/open:translate-x-0.5"
                  />
                </button>
              </div>
            </article>
          </div>
        </section>
      </div>
    </main>
  </div>
</template>
