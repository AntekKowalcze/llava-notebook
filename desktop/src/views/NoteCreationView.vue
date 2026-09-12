<script setup lang="ts">
import SubmitButton from '../components/commons/SubmitButton.vue';
import IconComponent from '../components/main/IconComponent.vue';
import { LockKeyhole, Cloud } from 'lucide-vue-next';
import SwitchInput from '../components/settings/SwitchInput.vue';
import TextInput from '../components/auth/forms/TextInput.vue';
import { InputTypes } from '../types/inputTypes';
import { ref, onMounted, computed } from 'vue';
import { useToast } from 'vue-toastification';
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

function settingChanged(id: string, value: string) {
  if (id === 'sync') {
    sync.value = value;
  }
  if (id === 'encryption') {
    encryption.value = value;
  }
}
function getErrorText(err: unknown): string {
  if (typeof err === 'string') {
    return err;
  }

  if (err && typeof err === 'object') {
    const typedErr = err as {
      message?: unknown;
      error?: unknown;
      reason?: unknown;
    };

    if (typeof typedErr.message === 'string') {
      return typedErr.message;
    }

    if (typeof typedErr.error === 'string') {
      return typedErr.error;
    }

    if (typeof typedErr.reason === 'string') {
      return typedErr.reason;
    }
  }

  return String(err ?? '');
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

    const message = getErrorText(err).toLowerCase();

    if (message.includes('note name already exists')) {
      toast.warning('A note with this name already exists.');
    } else if (message.includes('note name after sanitization is empty')) {
      toast.warning('The note title is invalid.');
    } else if (message.includes('title too long')) {
      toast.warning('The note title is too long.');
    } else if (message.includes('encryption key is unavailable')) {
      toast.error('Encryption key is unavailable.');
    } else if (message.includes('file operation error')) {
      toast.error('Failed to create the note file.');
    } else if (message.includes("couldn't lock state")) {
      toast.error('Failed to access application state.');
    } else if (message.includes('internal error')) {
      toast.error('An internal error occurred while creating the note.');
    } else {
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

    <div class="w-full max-w-4xl shrink-0 py-2">
      <div class="flex justify-center">
        <IconComponent
          width="w-[16vh] min-w-[8rem] max-w-[12rem]"
          height="h-[16vh] min-h-[8rem] max-h-[12rem]"
          class="object-contain"
        />
      </div>

      <!-- Heading -->
      <div class="mt-[2.5vh] text-center">
        <h1
          class="mx-auto max-w-2xl text-2xl font-bold leading-snug text-note-pumice sm:text-3xl md:text-4xl md:leading-relaxed"
        >
          <span class="text-note-paprika">Your ideas</span>
          deserve a place to stay.
          <br />
          Give them a home.
        </h1>
      </div>

      <div
        class="mt-[3.5vh] rounded-2xl border border-note-pumice/10 bg-black/40 px-6 py-5 shadow-2xl backdrop-blur-2xl sm:rounded-3xl sm:px-8 sm:py-7 md:px-10 md:py-8"
      >
        <div>
          <label
            class="mb-2 block text-xs uppercase tracking-[0.25em] text-note-pumice/50 sm:mb-3 sm:text-sm"
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

        <div class="mt-[3.5vh] space-y-3 sm:space-y-4">
          <div
            class="flex items-center justify-between rounded-xl border border-note-pumice/10 bg-black/30 px-5 py-3 sm:rounded-2xl sm:px-6 sm:py-5"
          >
            <div class="flex items-center gap-4 sm:gap-5">
              <div
                class="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-note-paprika/10 sm:h-14 sm:w-14"
              >
                <LockKeyhole class="h-6 w-6 text-note-paprika sm:h-7 sm:w-7" />
              </div>

              <div>
                <p class="text-base text-note-ivory sm:text-lg">Encryption</p>
                <p class="text-[11px] text-note-pumice/50 sm:text-sm">
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

          <div
            class="flex items-center justify-between rounded-xl border border-note-pumice/10 bg-black/30 px-5 py-3 sm:rounded-2xl sm:px-6 sm:py-5"
          >
            <div class="flex items-center gap-4 sm:gap-5">
              <div
                class="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-note-glow/10 sm:h-14 sm:w-14"
              >
                <Cloud class="h-6 w-6 text-note-glow sm:h-7 sm:w-7" />
              </div>

              <div>
                <p class="text-base text-note-ivory sm:text-lg">Synchronization</p>
                <p class="text-[11px] text-note-pumice/50 sm:text-sm">
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

        <SubmitButton
          content="Create note"
          class="mt-[3.5vh] h-12 w-full text-lg active:scale-[98%] sm:h-14 sm:text-xl"
          @click="createNote"
        />
      </div>
    </div>

    <div class="min-h-[1rem] w-full shrink grow-[1]"></div>
  </div>
</template>
