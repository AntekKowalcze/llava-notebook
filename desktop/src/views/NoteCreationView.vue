<script setup lang="ts">
import SubmitButton from '../components/commons/SubmitButton.vue';
import IconComponent from '../components/main/IconComponent.vue';
import { LockKeyhole, Cloud } from 'lucide-vue-next';
import SwitchInput from '../components/settings/SwitchInput.vue';
import TextInput from '../components/auth/forms/TextInput.vue';
import { InputTypes } from '../types/inputTypes';
import { ref, onMounted, computed, watch } from 'vue';
import { useToast } from 'vue-toastification';
import { errorKey } from '../lib/errors';
import { invoke } from '@tauri-apps/api/core';
import { Note } from '../types/note.ts';
import { useCurrentNoteStore } from '../stores/currentNoteStore.ts';
import { useRouter } from 'vue-router';
import { useUserConfigStore } from '../stores/userConfig.ts';
import DisabledSwitch from '../components/settings/DisabledSwitch.vue';
import { emit } from '@tauri-apps/api/event';
const router = useRouter();
const toast = useToast();
const userSettings = useUserConfigStore();
const syncStatus = computed(() => {
  const { 'online.sync': onlineSync, 'local.mode': localMode } = userSettings.config;

  if (localMode === 'on') {
    return {
      canSync: false,
      whyDisabled: 'You are in offline mode.',
    };
  }

  return onlineSync == 'on'
    ? { canSync: true, whyDisabled: '' }
    : {
        canSync: false,
        whyDisabled:
          'You are in online mode and online sync is turned off. Enable it in settings if you want to sync.',
      };
});

const canSync = computed(() => syncStatus.value.canSync);
const whyDisabled = computed(() => syncStatus.value.whyDisabled);

const currentNoteStore = useCurrentNoteStore();
const sync = ref<string>(userSettings.config['online.sync']);
const encryption = ref<string>(userSettings.config['local.encryption']);
const title = ref<string>('');

onMounted(async () => {
  await userSettings.init();
});

// Follow config changes (late load, `config-updated`) until the user picks a
// value on this page themselves.
const touched = new Set<string>();
watch(
  () => [userSettings.config['online.sync'], userSettings.config['local.encryption']],
  ([onlineSync, localEncryption]) => {
    if (!touched.has('sync')) sync.value = onlineSync;
    if (!touched.has('encryption')) encryption.value = localEncryption;
  }
);

function settingChanged(id: string, value: string) {
  touched.add(id);
  if (id === 'sync') {
    sync.value = value;
  }
  if (id === 'encryption') {
    encryption.value = value;
  }
}
async function createNote(): Promise<void> {
  if (title.value.trim().length === 0) {
    toast.warning('Title cannot be empty');
    return;
  }

  const useEncryption = encryption.value === 'on';
  const useSynchronization = canSync.value && sync.value === 'on';

  try {
    const createdNote = await invoke<Note>('create_note', {
      title: title.value.trim(),
      encryption: useEncryption,
      synchronizing: useSynchronization,
    });

    currentNoteStore.$patch({
      currentNote: createdNote,
    });
    await emit('reload-left-panel');

    toast.success('Note created successfully');

    await router.push(`/main/editor/${createdNote.local_id}`);
  } catch (err: unknown) {
    console.error('Failed to create note:', err);

    switch (errorKey(err)) {
      case 'NoteNameExistsError':
      case 'FileAlreadyExists':
        toast.warning('A note with this name already exists.');
        break;
      case 'NoteNameError':
        toast.warning('The note title is invalid.');
        break;
      case 'TitleTooLong':
      case 'NoteNameTooLong':
        toast.warning('The note title is too long.');
        break;
      case 'NoKeyToDecryptANote':
        toast.error('Encryption key is unavailable.');
        break;
      case 'FileOperationError':
        toast.error('Failed to create the note file.');
        break;
      case 'LockError':
        toast.error('Failed to access application state.');
        break;
      case 'InternalError':
        toast.error('An internal error occurred while creating the note.');
        break;
      default:
        toast.error('Failed to create note.');
    }
  }
}
</script>
<template>
  <div
    class="flex h-full w-full flex-col items-center overflow-y-auto bg-note-graphite px-4 [-ms-overflow-style:none] [scrollbar-width:none] sm:px-8 [&::-webkit-scrollbar]:hidden"
  >
    <div class="min-h-0 w-full shrink grow-[1.5]"></div>

    <div class="w-full max-w-2xl shrink-0 py-2 min-[1600px]:max-w-4xl">
      <!-- Responsive Icon Sizing -->
      <div class="flex justify-center">
        <IconComponent
          width="w-[12vh] min-w-[5.5rem] max-w-[8rem] min-[1600px]:w-[16vh] min-[1600px]:min-w-[8rem] min-[1600px]:max-w-[12rem]"
          height="h-[12vh] min-h-[5.5rem] max-h-[8rem] min-[1600px]:h-[16vh] min-[1600px]:min-h-[8rem] min-[1600px]:max-h-[12rem]"
          class="object-contain"
        />
      </div>

      <!-- Responsive Heading -->
      <div class="mt-[2vh] text-center min-[1600px]:mt-[2.5vh]">
        <h1
          class="mx-auto max-w-2xl text-xl font-bold leading-snug text-note-pumice sm:text-2xl min-[1600px]:text-4xl min-[1600px]:leading-relaxed"
        >
          <span class="text-note-paprika">Your ideas</span>
          deserve a place to stay.
          <br />
          Give them a home.
        </h1>
      </div>

      <!-- Responsive Form Card -->
      <div
        class="mt-[2.5vh] rounded-2xl border border-note-pumice/10 bg-black/40 px-5 py-5 shadow-2xl backdrop-blur-2xl sm:rounded-3xl sm:px-6 sm:py-6 min-[1600px]:mt-[3.5vh] min-[1600px]:p-10"
      >
        <div>
          <label
            class="mb-1.5 block text-xs uppercase tracking-[0.25em] text-note-pumice/50 sm:mb-2 sm:text-sm"
          >
            Note title
          </label>

          <TextInput
            name=""
            placeholder="Give your note a title"
            :type="InputTypes.Text"
            class="w-full"
            v-model="title"
          ></TextInput>
        </div>

        <!-- Responsive Settings Stack -->
        <div
          class="mt-[2.5vh] space-y-2.5 sm:space-y-3 min-[1600px]:mt-[3.5vh] min-[1600px]:space-y-4"
        >
          <!-- Encryption Row -->
          <div
            class="flex items-center justify-between rounded-xl border border-note-pumice/10 bg-black/30 px-4 py-3 sm:rounded-2xl sm:px-5 sm:py-3.5 min-[1600px]:px-6 min-[1600px]:py-5"
          >
            <div class="flex items-center gap-3 sm:gap-4 min-[1600px]:gap-5">
              <div
                class="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-note-paprika/10 sm:h-11 sm:w-11 min-[1600px]:h-14 min-[1600px]:w-14"
              >
                <LockKeyhole
                  class="h-5 w-5 text-note-paprika sm:h-6 sm:w-6 min-[1600px]:h-7 min-[1600px]:w-7"
                />
              </div>

              <div>
                <p class="text-sm text-note-ivory sm:text-base min-[1600px]:text-lg">Encryption</p>
                <p class="text-[10px] text-note-pumice/50 sm:text-xs min-[1600px]:text-sm">
                  Protect your private thoughts
                </p>
              </div>
            </div>
            <SwitchInput
              :current-value="encryption"
              id="encryption"
              class="shrink-0"
              @setting-changed="settingChanged"
            />
          </div>

          <!-- Sync Row -->
          <div
            class="flex items-center justify-between rounded-xl border border-note-pumice/10 bg-black/30 px-4 py-3 sm:rounded-2xl sm:px-5 sm:py-3.5 min-[1600px]:px-6 min-[1600px]:py-5"
          >
            <div class="flex items-center gap-3 sm:gap-4 min-[1600px]:gap-5">
              <div
                class="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-note-glow/10 sm:h-11 sm:w-11 min-[1600px]:h-14 min-[1600px]:w-14"
              >
                <Cloud
                  class="h-5 w-5 text-note-glow sm:h-6 sm:w-6 min-[1600px]:h-7 min-[1600px]:w-7"
                />
              </div>

              <div>
                <p class="text-sm text-note-ivory sm:text-base min-[1600px]:text-lg">
                  Synchronization
                </p>
                <p class="text-[10px] text-note-pumice/50 sm:text-xs min-[1600px]:text-sm">
                  Keep your knowledge everywhere
                </p>
              </div>
            </div>

            <SwitchInput
              :current-value="sync"
              id="sync"
              class="shrink-0"
              @setting-changed="settingChanged"
              v-if="canSync"
            />
            <DisabledSwitch
              id="sync"
              class="shrink-0"
              :why-disabled="whyDisabled"
              v-else
              :checked="false"
            />
          </div>
        </div>

        <!-- Responsive Submit Button -->
        <SubmitButton
          content="Create note"
          class="mt-[2.5vh] h-11 w-full text-base active:scale-[98%] sm:h-12 min-[1600px]:mt-[3.5vh] min-[1600px]:h-14 min-[1600px]:text-xl"
          @click="createNote"
        />
      </div>
    </div>

    <div class="min-h-[1rem] w-full shrink grow-[1]"></div>
  </div>
</template>
