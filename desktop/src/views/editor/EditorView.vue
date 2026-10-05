<script setup lang="ts">
import { MilkdownProvider } from '@milkdown/vue';
import '@milkdown/crepe/theme/common/style.css';
import { ArrowBigLeftDash } from 'lucide-vue-next';
import { useRoute, useRouter, onBeforeRouteUpdate, onBeforeRouteLeave } from 'vue-router';
import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import LoadingCircle from '../../components/main/LoadingCircle.vue';
import '../../css/milkdown.css';
import MilkdownEditor from '../../components/editor/MilkdownEditor.vue';
import PlusButton from '../../components/editor/PlusButton.vue';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useToast } from 'vue-toastification';
import { useCurrentNoteStore } from '../../stores/currentNoteStore.ts';
import { Note } from '../../types/note.ts';
import { errorKey } from '../../lib/errors';
import { useLayoutStore } from '../../stores/layoutStore.ts';
import TagEdition from '../../components/editor/tagEdition.vue';
import NoteHeader from '../../components/editor/NoteHeader.vue';
import QuitModal from '../../components/editor/QuitModal.vue';
import { useShortcuts } from '../../lib/shortcuts';
let leftPanelReloaded = ref<boolean>(false);
const currentNoteStore = useCurrentNoteStore();
let isDirty = false;
let isClosing = false;
let unlistenClose: (() => void) | null = null;
let unlistenSyncFinished: UnlistenFn | null = null;

const showForceQuitModal = ref(false);
const saveError = ref('');

const DEBOUNCE_TIME = 2000;
const SAFE_SAVE = 60000;

const router = useRouter();
const route = useRoute();
const toast = useToast();

const noteId = computed(() => route.params.noteId as string);

const noteContent = ref<string>('');
const isLoading = ref<boolean>(true);
// True when the stored note could not be read. The editor must not be shown
// then: its placeholder text would be saved over the real note, and leaving
// the page would delete the attachments the (unread) content references.
const loadFailed = ref<boolean>(false);

const date = ref(new Date());
const h = computed(() => date.value.getHours());
const layoutStore = useLayoutStore();
let safeSaveTimeout: ReturnType<typeof setTimeout> | null = null;
let debounceTimeout: ReturnType<typeof setTimeout> | null = null;
let encryptionChangedTo: boolean | null = null;
// Bumped to rebuild the editor when its content is replaced from outside.
const editorVersion = ref(0);

/** Points the attachment links in `content` at the ids they were replaced with. */
function replaceAttachmentIds(content: string, replaced: [string, string][]): string {
  for (const [oldId, newId] of replaced) {
    content = content.split(oldId).join(newId);
  }
  return content;
}

async function forceQuit() {
  await getCurrentWindow().destroy();
}

async function retrySave() {
  showForceQuitModal.value = false;

  const success = await saveNote();

  if (!success) {
    showForceQuitModal.value = true;
  }
}

const defaultValue = computed(() => {
  if (h.value < 6) {
    return '# Deep night thoughts? 🌌\n\n> The rest of the world is sleeping, but your mind is awake.\n\nJot down your late-night inspiration before it fades...';
  }

  if (h.value < 12) {
    return '# Good morning! ☀️\n\nA fresh day, a blank canvas. **What are we focusing on today?**\n\n- [ ] Task 1\n- [ ] Task 2';
  }

  if (h.value < 18) {
    return '# Good afternoon! ☕\n\nMid-day inspiration striking?\n\nDrop your notes, ideas, or meeting summaries right here.';
  }

  if (h.value < 22) {
    return "# Good evening! 🌇\n\nWinding down? It's the perfect time to reflect on the day or plan for tomorrow.";
  }

  return "# Entering Stealth Mode 🥷\n\nDistractions are asleep. It's just you, the keyboard, and the glow of the screen.\n\n**Execute your final thoughts for the day:**";
});

async function loadNoteContent(id: string): Promise<string> {
  leftPanelReloaded.value = false;
  try {
    const contentFromDb = await invoke<string>('get_note_content', {
      noteId: id,
    });
    if (currentNoteStore.currentNote?.local_id !== id) {
      currentNoteStore.currentNote = await invoke<Note>('get_note_object', { noteId: id });
    }

    loadFailed.value = false;

    if (contentFromDb.trim() === '') {
      return defaultValue.value;
    }

    return contentFromDb;
  } catch (err) {
    console.error('Failed to load note:', err);
    toast.error('failed to get note content');
    loadFailed.value = true;
    return '';
  }
}

async function retryLoad() {
  isLoading.value = true;

  try {
    noteContent.value = await loadNoteContent(noteId.value);
    setWordCount();
  } finally {
    isLoading.value = false;
  }
}

function setDebounceTimeout() {
  if (!debounceTimeout) {
    debounceTimeout = setTimeout(async () => {
      await saveNote();
    }, DEBOUNCE_TIME);
  }
}

function setSafeSaveTimeout() {
  if (!safeSaveTimeout) {
    safeSaveTimeout = setTimeout(async () => {
      await saveNote();
    }, SAFE_SAVE);
  }
}

type SyncFinishedPayload = { changed_note_ids: string[]; removed_note_ids: string[] };

// A sync may have replaced or deleted the note that is open. Without this the
// editor keeps showing the old text and the next autosave would silently
// overwrite the newer cloud version.
async function handleSyncFinished(payload: SyncFinishedPayload) {
  const id = noteId.value;

  if (isLoading.value || loadFailed.value) {
    return;
  }

  if (payload.removed_note_ids.includes(id)) {
    toast.warning('This note was deleted on another device.');
    if (!isDirty) {
      redirect();
    }
    return;
  }

  if (!payload.changed_note_ids.includes(id)) {
    return;
  }

  if (isDirty) {
    toast.warning(
      'This note was changed on another device. Saving here will overwrite that change.',
      { timeout: 10000 }
    );
    return;
  }

  // Only the text changed; the open note's metadata stays (loadNoteContent
  // would also replace currentNote, which is wrong once the user has moved on).
  let content: string;
  try {
    content = await invoke<string>('get_note_content', { noteId: id });
  } catch (err) {
    console.error('Failed to reload note after sync:', err);
    return;
  }

  // The user may have opened another note, or started editing, while this
  // reload ran; the reloaded text must not land in that note.
  if (noteId.value !== id || isDirty || isLoading.value) {
    return;
  }

  noteContent.value = content.trim() === '' ? defaultValue.value : content;
  setWordCount();
  editorVersion.value++;
}

onMounted(async () => {
  const tauriWindow = getCurrentWindow();

  unlistenSyncFinished = await listen<SyncFinishedPayload>('sync_finished', (event) => {
    void handleSyncFinished(event.payload);
  });

  unlistenClose = await tauriWindow.onCloseRequested(async (event) => {
    if (isClosing) {
      return;
    }

    if (isDirty) {
      event.preventDefault();

      const success = await saveNote();

      if (success) {
        isClosing = true;
        await tauriWindow.close();
      } else {
        showForceQuitModal.value = true;
      }
    }
  });

  try {
    noteContent.value = await loadNoteContent(noteId.value);
    setWordCount();
  } catch (err) {
    console.error(err);
  } finally {
    isLoading.value = false;
  }
});

onUnmounted(() => {
  if (unlistenSyncFinished) {
    unlistenSyncFinished();
    unlistenSyncFinished = null;
  }
  if (unlistenClose) {
    unlistenClose();
    unlistenClose = null;
  }
  if (loadFailed.value) {
    return;
  }
  void invoke('clean_attachments', {
    content: noteContent.value,
  }).catch((err) => {
    console.error('Failed to clean attachments:', err);
  });
});

onBeforeRouteUpdate(async (to) => {
  if (!isDirty) {
    isLoading.value = true;

    try {
      noteContent.value = await loadNoteContent(to.params.noteId as string);
      setWordCount();
    } finally {
      isLoading.value = false;
    }

    return true;
  }

  isLoading.value = true;

  const oldNoteId = noteId.value;

  const success = await saveNote(oldNoteId);

  if (!success) {
    isLoading.value = false;
    return false;
  }

  isDirty = false;
  encryptionChangedTo = null;

  if (debounceTimeout) {
    clearTimeout(debounceTimeout);
    debounceTimeout = null;
  }

  if (safeSaveTimeout) {
    clearTimeout(safeSaveTimeout);
    safeSaveTimeout = null;
  }

  try {
    noteContent.value = await loadNoteContent(to.params.noteId as string);
    return true;
  } finally {
    isLoading.value = false;
  }
});

onBeforeRouteLeave(async () => {
  if (!isDirty) {
    return true;
  }

  isLoading.value = true;

  const success = await saveNote(noteId.value);

  isLoading.value = false;

  return success;
});

function contentChanged(content: string) {
  noteContent.value = content;
  setWordCount(content);
  if (!isDirty) {
    setSafeSaveTimeout();
    isDirty = true;
  }

  if (debounceTimeout) {
    clearTimeout(debounceTimeout);
    debounceTimeout = null;
  }

  setDebounceTimeout();
}

// Moving to trash needs a second press within this window.
const TRASH_CONFIRM_MS = 4000;
let trashArmedUntil = 0;

async function moveToTrash() {
  const note = currentNoteStore.currentNote;
  if (!note || loadFailed.value) return;

  if (Date.now() > trashArmedUntil) {
    trashArmedUntil = Date.now() + TRASH_CONFIRM_MS;
    toast.info('Press Ctrl+Shift+Delete again to move this note to trash', {
      timeout: TRASH_CONFIRM_MS,
    });
    return;
  }
  trashArmedUntil = 0;

  if (isDirty && !(await saveNote())) return;

  try {
    await invoke<void>('remove_note', { noteId: note.local_id });
  } catch (err) {
    console.error('Failed to move note to trash:', err);
    toast.error('Failed to move note to trash');
    return;
  }

  isDirty = false;
  if (debounceTimeout) clearTimeout(debounceTimeout);
  if (safeSaveTimeout) clearTimeout(safeSaveTimeout);
  debounceTimeout = null;
  safeSaveTimeout = null;

  await emit('reload-left-panel');
  toast.success('Note moved to trash');
  router.push({ name: 'allNotes' });
}

function toggleEncryption() {
  const note = currentNoteStore.currentNote;
  if (!note || loadFailed.value) return;

  const to = !note.encrypted;
  changeEncMethod(to);
  setDebounceTimeout();
  toast.info(to ? 'Encrypting this note…' : 'Turning off encryption for this note…', {
    timeout: 2500,
  });
}

useShortcuts({
  save: () => void saveNote(),
  rename: () => !loadFailed.value && layoutStore.requestTitleEdit(),
  tags: () => currentNoteStore.currentNote && !loadFailed.value && layoutStore.openTagEditor(),
  toggleEncryption,
  moveToTrash: () => void moveToTrash(),
  back: redirect,
});

function redirect() {
  router.push({ name: 'create' });
}

async function saveNote(id: string = noteId.value): Promise<boolean> {
  if (loadFailed.value) {
    return true;
  }

  try {
    if (!leftPanelReloaded.value) {
      await emit('reload-left-panel');
      leftPanelReloaded.value = true;
    }
    const sentContent = noteContent.value;
    const sentEncryption = encryptionChangedTo;
    // Switching encryption stores the attachments as new copies (new ids).
    const replaced = Object.entries(
      await invoke<Record<string, string>>('save_note', {
        noteId: id,
        content: sentContent,
        nextSaveToEncryption: sentEncryption,
      })
    );
    const savedContent = replaceAttachmentIds(sentContent, replaced);
    if (replaced.length > 0 && id === noteId.value) {
      noteContent.value = replaceAttachmentIds(noteContent.value, replaced);
      editorVersion.value++;
    }
    await emit('note-saved');

    if (currentNoteStore.currentNote && currentNoteStore.currentNote.local_id === id) {
      currentNoteStore.currentNote.updated_at = Date.now();
    }
    // A toggle made while the save was running is still pending: keep it
    // (the switch already shows it) and save it next.
    if (sentEncryption != null && encryptionChangedTo === sentEncryption) {
      if (currentNoteStore.currentNote) {
        currentNoteStore.currentNote.encrypted = sentEncryption;
      }

      encryptionChangedTo = null;
    }

    if (debounceTimeout) {
      clearTimeout(debounceTimeout);
      debounceTimeout = null;
    }

    if (safeSaveTimeout) {
      clearTimeout(safeSaveTimeout);
      safeSaveTimeout = null;
    }

    // Edits typed while the save was running still need saving.
    isDirty = noteContent.value !== savedContent || encryptionChangedTo !== null;
    if (isDirty) setDebounceTimeout();

    return true;
  } catch (err: unknown) {
    console.error('Failed to save note:', err);

    switch (errorKey(err)) {
      case 'UserIsNotOwner':
        toast.error("You don't have permission to save this note.");
        break;
      case 'NoteNotFound':
        toast.warning('This note no longer exists.');
        break;
      case 'NoKeyToDecryptANote':
        toast.error('Encryption key is unavailable.');
        break;
      case 'FileOperationError':
        toast.error('Failed to save the note to disk.');
        break;
      case 'LockError':
        toast.error('Failed to access application state.');
        break;
      case 'InternalError':
        toast.error('An internal error occurred while saving the note.');
        break;
      default:
        toast.error('Failed to save note.');
    }

    isDirty = true;

    return false;
  }
}

function changeEncMethod(to: boolean) {
  if (currentNoteStore.currentNote) {
    currentNoteStore.currentNote.encrypted = to;
  }
  encryptionChangedTo = to;
  isDirty = true;
}
function setWordCount(text = noteContent.value) {
  const cleaned = text.replace(/&#x20;/g, ' ').trim();

  currentNoteStore.words = cleaned ? cleaned.split(/\s+/).length : 0;
}
</script>
<template>
  <QuitModal
    :visible="showForceQuitModal"
    :message="saveError"
    @close="showForceQuitModal = false"
    @retry="retrySave"
    @force-quit="forceQuit"
  />
  <TagEdition
    v-if="layoutStore.isTagEditorOpen"
    :note-id="currentNoteStore.currentNote!.local_id"
    @close="layoutStore.closeTagEditor()"
  />

  <LoadingCircle
    v-if="isLoading"
    class="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2"
  />

  <div
    v-else-if="loadFailed"
    class="flex h-full flex-1 flex-col items-center justify-center gap-4 text-note-ivory"
  >
    <p>This note could not be loaded. Nothing was changed.</p>
    <div class="flex gap-4">
      <button
        type="button"
        class="rounded-md border border-note-paprika px-4 py-1 hover:bg-note-graphite/60"
        @click="retryLoad"
      >
        Try again
      </button>
      <button
        type="button"
        class="rounded-md border border-note-paprika px-4 py-1 hover:bg-note-graphite/60"
        @click="redirect"
      >
        Back
      </button>
    </div>
  </div>

  <div
    v-else
    class="relative flex h-full min-h-0 flex-1 flex-col overflow-hidden"
  >
    <ArrowBigLeftDash
      class="absolute left-[2%] top-[93%] z-[100] cursor-pointer text-note-paprika/80 transition-transform duration-200 hover:scale-95 active:scale-90"
      @click="redirect"
    />

    <div class="absolute bottom-16 right-8 z-[110]">
      <PlusButton @change_encryption_method="changeEncMethod" />
    </div>

    <div class="flex h-full min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
      <div class="relative h-full min-h-0 min-w-0 flex-1 overflow-y-auto">
        <MilkdownProvider>
          <MilkdownEditor
            :key="`${noteId}-${editorVersion}`"
            :note-id="noteId"
            :default-value="noteContent"
            @change="contentChanged"
          />
        </MilkdownProvider>
      </div>
    </div>
    <NoteHeader
      v-if="currentNoteStore.currentNote"
      :note-id="currentNoteStore.currentNote.local_id"
      :note-name="currentNoteStore.currentNote.title"
    />
  </div>
</template>
