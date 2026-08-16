<script setup lang="ts">
import { MilkdownProvider } from '@milkdown/vue'
import '@milkdown/crepe/theme/common/style.css'
import { ArrowBigLeftDash } from 'lucide-vue-next'
import {
  useRoute,
  useRouter,
  onBeforeRouteUpdate,
  onBeforeRouteLeave
} from 'vue-router'
import { emit } from '@tauri-apps/api/event'
import { computed, onMounted, onUnmounted, ref } from 'vue'
import LoadingCircle from '../../components/main/LoadingCircle.vue'
import '../../css/milkdown.css'
import MilkdownEditor from '../../components/editor/MilkdownEditor.vue'
import PlusButton from '../../components/editor/PlusButton.vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useToast } from 'vue-toastification'
import { useCurrentNoteStore } from '../../stores/currentNoteStore.ts'
import { Note } from '../../types/note.ts'
import { useLayoutStore } from '../../stores/layoutStore.ts'
import TagEdition from '../../components/editor/tagEdition.vue'
const currentNoteStore = useCurrentNoteStore()
import NoteHeader from '../../components/editor/NoteHeader.vue'

let isDirty = false
let isClosing = false
let unlistenClose: (() => void) | null = null

const DEBOUNCE_TIME = 2000
const SAFE_SAVE = 60000

const router = useRouter()
const route = useRoute()
const toast = useToast()

const noteId = computed(() => route.params.noteId as string)

const noteContent = ref<string>('')
const isLoading = ref<boolean>(true)

const date = ref(new Date())
const h = computed(() => date.value.getHours())
const layoutStore = useLayoutStore();
let safeSaveTimeout: ReturnType<typeof setTimeout> | null = null
let debounceTimeout: ReturnType<typeof setTimeout> | null = null
let encryptionChangedTo: boolean | null = null

const showTagEditor = ref(false);

function addTag() {
  showTagEditor.value = true;
}

const defaultValue = computed(() => {
  if (h.value < 6) {
    return "# Deep night thoughts? 🌌\n\n> The rest of the world is sleeping, but your mind is awake.\n\nJot down your late-night inspiration before it fades..."
  }

  if (h.value < 12) {
    return "# Good morning! ☀️\n\nA fresh day, a blank canvas. **What are we focusing on today?**\n\n- [ ] Task 1\n- [ ] Task 2"
  }

  if (h.value < 18) {
    return "# Good afternoon! ☕\n\nMid-day inspiration striking?\n\nDrop your notes, ideas, or meeting summaries right here."
  }

  if (h.value < 22) {
    return "# Good evening! 🌇\n\nWinding down? It's the perfect time to reflect on the day or plan for tomorrow."
  }

  return "# Entering Stealth Mode 🥷\n\nDistractions are asleep. It's just you, the keyboard, and the glow of the screen.\n\n**Execute your final thoughts for the day:**"
})

async function loadNoteContent(id: string): Promise<string> {
  try {
    const contentFromDb = await invoke<string>("get_note_content", {
      noteId: id,
    })
if (currentNoteStore.currentNote?.local_id !== id) {
     currentNoteStore.currentNote = await invoke<Note>("get_note_object", { noteId: id })
   }
    if (currentNoteStore.currentNote?.local_id !== id) {
      currentNoteStore.currentNote = await invoke("get_note", { noteId: id })
    }
 
    if (contentFromDb.trim() === '') {
      return defaultValue.value
    }

    return contentFromDb
  } catch (err) {
    console.log(err)
    toast.error("failed to get note content")
    return defaultValue.value
  }
}

function setDebounceTimeout() {
  if (!debounceTimeout) {
    debounceTimeout = setTimeout(async () => {
      await saveNote()
    }, DEBOUNCE_TIME)
  }
}

function setSafeSaveTimeout() {
  if (!safeSaveTimeout) {
    safeSaveTimeout = setTimeout(async () => {
      await saveNote()
    }, SAFE_SAVE)
  }
}

onMounted(async () => {
  const tauriWindow = getCurrentWindow()

  unlistenClose = await tauriWindow.onCloseRequested(async (event) => {
    if (isClosing) {
      return
    }

    if (isDirty) {
      event.preventDefault()

      const success = await saveNote()

      if (success) {
        isClosing = true
        await tauriWindow.close()
      }
    }
  })

  try {
    noteContent.value = await loadNoteContent(noteId.value)
    setWordCount();
    console.log("LOADED:", JSON.stringify(noteContent.value))
  } catch (err) {
    console.error(err)
  } finally {
    isLoading.value = false
  }

  window.addEventListener('keydown', handleKeyDown)
})

onUnmounted(() => {
  if (unlistenClose) {
    unlistenClose()
    unlistenClose = null
  }

  window.removeEventListener('keydown', handleKeyDown)
})

onBeforeRouteUpdate(async (to) => {
  if (!isDirty) {
    isLoading.value = true

    try {
      noteContent.value = await loadNoteContent(to.params.noteId as string)
      setWordCount();
    } finally {
      isLoading.value = false
    }

    return true
  }

  isLoading.value = true

  const oldNoteId = noteId.value

  const success = await saveNote(oldNoteId)

  if (!success) {
    isLoading.value = false
    return false
  }

  isDirty = false
  encryptionChangedTo = null

  if (debounceTimeout) {
    clearTimeout(debounceTimeout)
    debounceTimeout = null
  }

  if (safeSaveTimeout) {
    clearTimeout(safeSaveTimeout)
    safeSaveTimeout = null
  }

  try {
    noteContent.value = await loadNoteContent(to.params.noteId as string)
    return true
  } finally {
    isLoading.value = false
  }
})

onBeforeRouteLeave(async () => {
  if (!isDirty) {
    return true
  }

  isLoading.value = true

  const success = await saveNote(noteId.value)

  isLoading.value = false

  return success
})

function contentChanged(content: string) {
  noteContent.value = content

  if (!isDirty) {
    setSafeSaveTimeout()
    isDirty = true
  }

  if (debounceTimeout) {
    clearTimeout(debounceTimeout)
    debounceTimeout = null
  }

  setDebounceTimeout()
}

async function handleKeyDown(event: KeyboardEvent) {
  if (event.ctrlKey && event.key.toLowerCase() === 's') {
    event.preventDefault()
    await saveNote()
  }

}

function redirect() {
  router.push({ name: 'create' })
}

function getErrorText(err: unknown): string {
  if (typeof err === 'string') {
    return err
  }

  if (err && typeof err === 'object') {
    const typedErr = err as {
      message?: unknown
      error?: unknown
      reason?: unknown
    }

    if (typeof typedErr.message === 'string') {
      return typedErr.message
    }

    if (typeof typedErr.error === 'string') {
      return typedErr.error
    }

    if (typeof typedErr.reason === 'string') {
      return typedErr.reason
    }
  }

  return String(err ?? '')
}

async function saveNote(id: string = noteId.value): Promise<boolean> {
  try {
    await invoke("save_note", {
      noteId: id,
      content: noteContent.value,
      nextSaveToEncryption: encryptionChangedTo
    })
    await emit('note-saved');
 if (currentNoteStore.currentNote && currentNoteStore.currentNote.local_id === id) {
   currentNoteStore.currentNote.updated_at = Date.now()
 }
    if (encryptionChangedTo != null) {
      if (currentNoteStore.currentNote) {
        currentNoteStore.currentNote.encrypted = encryptionChangedTo
      }

      encryptionChangedTo = null
    }

    if (debounceTimeout) {
      clearTimeout(debounceTimeout)
      debounceTimeout = null
    }

    if (safeSaveTimeout) {
      clearTimeout(safeSaveTimeout)
      safeSaveTimeout = null
    }

    isDirty = false

    return true

  } catch (err: unknown) {
    console.error("Failed to save note:", err)

    const message = getErrorText(err).toLowerCase()

    if (message.includes("user is not owner")) {
      toast.error("You don't have permission to save this note.")
    } else if (message.includes("note not found")) {
      toast.warning("This note no longer exists.")
    } else if (message.includes("encryption error")) {
      toast.error("Encryption key is unavailable.")
    } else if (message.includes("file operation error")) {
      toast.error("Failed to save the note to disk.")
    } else if (message.includes("couldn't lock state")) {
      toast.error("Failed to access application state.")
    } else if (message.includes("internal error")) {
      toast.error("An internal error occurred while saving the note.")
    } else {
      toast.error("Failed to save note.")
    }

    isDirty = true

    return false
  }
}

function changeEncMethod(to: boolean) {
   if (currentNoteStore.currentNote) {
    currentNoteStore.currentNote.encrypted = to
  }
  encryptionChangedTo = to
  isDirty = true
}
function setWordCount(text = noteContent.value) {
  const cleaned = text
    .replace(/&#x20;/g, ' ')
    .trim()

  currentNoteStore.words = cleaned
    ? cleaned.split(/\s+/).length
    : 0
}
</script>
<template>
<TagEdition
  v-if="layoutStore.isTagEditorOpen"
  :note-id="currentNoteStore.currentNote!.local_id"
  @close="layoutStore.closeTagEditor()"
/>
 
  <LoadingCircle
    v-if="isLoading"
    class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2"
  />

  <div
    v-else
    class="flex flex-1 min-h-0 flex-col"
  >
    <NoteHeader
      v-if="currentNoteStore.currentNote"
      :note-id="currentNoteStore.currentNote.local_id"
      :note-name="currentNoteStore.currentNote.title"
    />
    <ArrowBigLeftDash
      class="absolute left-[2%] top-[93%] text-note-paprika/80
             transition-transform duration-200
             hover:scale-95 active:scale-90"
      @click="redirect"
    />

<div class="absolute right-8 bottom-16 z-[110]">
  <PlusButton @change_encryption_method="changeEncMethod" />
</div>
    <div class="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
   <div class="min-h-0 min-w-0 flex-1 overflow-hidden">
    <MilkdownProvider>
      <MilkdownEditor
        :key="noteId"
        :default-value="noteContent"
        @change="contentChanged"
      />
    </MilkdownProvider>
  </div>
</div>
</div>
</template>

<!-- TODO zablokować w create note sync jeśli jest offline mode -->
<!-- TODO jeśli nie ma connected, zablokować przełącznik do offline mode off -->
<!-- TODO !!! when failed to save a note, nie da się wyjść, coś w stylu force save dać (to raczej się nie powinno zdarzyć) -->
 <!--   tytuł aktualnej notatki -->
  <!-- Jutro view wszystkich notatek -->
