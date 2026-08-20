<script setup lang="ts">
import { Star, Tag } from 'lucide-vue-next';
import ScreenDeviderHorizontal from '../dashboard/ScreenDeviderHorizontal.vue';
import SwitchInput from '../settings/SwitchInput.vue';
import { useCurrentNoteStore } from '../../stores/currentNoteStore.ts';
import { computed, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useToast } from 'vue-toastification';
import { useLayoutStore } from '../../stores/layoutStore';
import SubmitButton from '../commons/SubmitButton.vue';
import TextInput from '../auth/forms/TextInput.vue';
import { InputTypes } from '../../types/inputTypes.ts';
import { emit as emitTauri } from '@tauri-apps/api/event';
const layoutStore = useLayoutStore();
const toast = useToast();
const emit = defineEmits<{ (e: 'change_encryption_method', to: boolean): void }>();
const currentNoteStore = useCurrentNoteStore();
const isChangingTitle = ref(false);
const newTitle = ref('');
const encryptionState = computed(() => (currentNoteStore.currentNote?.encrypted ? 'on' : 'off'));

const syncState = computed(() =>
  currentNoteStore.currentNote?.sync_state === 'LocalOnly' ? 'off' : 'on'
);

async function settingChanged(id: string, value: string) {
  const note = currentNoteStore.currentNote;
  if (!note) return;

  const noteId = note.local_id;
  switch (id) {
    case 'encryption': {
      emit('change_encryption_method', value == 'on');
      break;
    }
    case 'sync': {
      try {
        await invoke<void>('toggle_note_sync', { noteId, value });

        if (currentNoteStore.currentNote) {
          currentNoteStore.currentNote.sync_state = value === 'off' ? 'LocalOnly' : 'PendingUpload';
        }
      } catch (err) {
        console.log(err);
        useToast().warning('Failed to change sync state');
      }
      break;
    }
  }
}

async function addToFavourites() {
  if (currentNoteStore.currentNote) {
    let currentNoteId = currentNoteStore.currentNote.local_id;
    try {
      invoke<void>('add_tag_to_note', {
        noteId: currentNoteId,
        tagName: 'favourites',
        tagColor: '#FACC15',
      });
      toast.success('Successfuly added to favourites');
      emitTauri('tags_changed');
    } catch (err) {
      console.log(err);
      useToast().warning('Failed to add note to favourites');
    }
  }
}

function changeTitle() {
  isChangingTitle.value = !isChangingTitle.value;

  if (isChangingTitle.value) {
    newTitle.value = currentNoteStore.currentNote?.title ?? '';
  }
}

async function submitTitle() {
  const note = currentNoteStore.currentNote;
  if (!note) return;

  const title = newTitle.value.trim();

  if (!title) {
    toast.warning('Title cannot be empty');
    return;
  }

  try {
    await invoke<void>('change_note_title', {
      noteId: note.local_id,
      title,
    });

    note.title = title;

    isChangingTitle.value = false;

    toast.success('Title changed successfully');
  } catch (err) {
    console.error(err);
    toast.warning('Failed to change title');
  }
}
</script>

<template>
  <div
    v-if="currentNoteStore.currentNote"
    class="absolute bottom-full right-full z-[50] w-60 rounded-lg border-2 border-note-paprika bg-black p-4 shadow-2xl"
  >
    <button
      type="button"
      class="group flex h-9 w-full items-center justify-between rounded-md px-3 text-left hover:bg-note-graphite/60"
      @click="addToFavourites"
    >
      <span class="text-sm font-medium text-note-ivory group-hover:text-note-paprika">
        Add to favourites
      </span>
      <Star class="h-4 w-4 text-note-glow group-hover:scale-110 group-hover:fill-note-glow" />
    </button>

    <ScreenDeviderHorizontal />

    <button
      type="button"
      class="group flex h-9 w-full items-center justify-between rounded-md px-3 text-left hover:bg-note-graphite/60"
      @click="layoutStore.openTagEditor()"
    >
      <span class="text-sm font-medium text-note-ivory group-hover:text-note-paprika">
        Manage tags
      </span>
      <Tag class="h-4 w-4 text-note-garnet group-hover:scale-110 group-hover:fill-note-garnet" />
    </button>

    <ScreenDeviderHorizontal />

    <button
      type="button"
      class="group flex h-9 w-full items-center justify-between rounded-md px-3 text-left hover:bg-note-graphite/60"
      @click="changeTitle"
    >
      <span class="text-sm font-medium text-note-ivory group-hover:text-note-paprika">
        Change Title
      </span>
    </button>

    <Transition
      enter-active-class="transition-all duration-200 ease-out"
      enter-from-class="max-h-0 opacity-0"
      enter-to-class="max-h-24 opacity-100"
      leave-active-class="transition-all duration-150 ease-in"
      leave-from-class="max-h-24 opacity-100"
      leave-to-class="max-h-0 opacity-0"
    >
      <div
        v-if="isChangingTitle"
        class="overflow-hidden px-3 pb-2"
      >
        <div class="flex flex-col gap-2">
          <div class="-mt-3">
            <TextInput
              :placeholder="'Input new title'"
              :type="InputTypes.Text"
              :name="'title'"
              v-model="newTitle"
              class="mt-0 w-full"
            />
          </div>

          <SubmitButton
            :disabled="false"
            :content="'Change title'"
            type="button"
            @click="submitTitle"
            class="mt-2 origin-left scale-[0.75]"
          />
        </div>
      </div>
    </Transition>

    <ScreenDeviderHorizontal />

    <div
      class="group flex h-9 w-full items-center justify-between rounded-md px-3 text-left hover:bg-note-graphite/60"
    >
      <span class="text-sm font-medium text-note-ivory group-hover:text-note-paprika">
        Note encryption
      </span>

      <SwitchInput
        class="scale-[0.8]"
        id="encryption"
        :current-value="encryptionState"
        @setting-changed="settingChanged"
      />
    </div>

    <ScreenDeviderHorizontal />

    <div
      class="group flex h-9 w-full items-center justify-between rounded-md px-3 text-left hover:bg-note-graphite/60"
    >
      <span class="text-sm font-medium text-note-ivory group-hover:text-note-paprika">
        Note synchronization
      </span>
      <SwitchInput
        class="scale-[0.8]"
        id="sync"
        :current-value="syncState"
        @setting-changed="settingChanged"
      />
    </div>
  </div>
</template>
