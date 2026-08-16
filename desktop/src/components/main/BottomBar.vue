<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useUserConfigStore } from '../../stores/userConfig';
import {
  Lock,
  CloudOff,
  LockOpen,
  HardDrive,
  Server,
  Cloud,
  RefreshCwOff,
  CloudCheck,
  Save
} from 'lucide-vue-next';
import { useMetaStore } from '../../stores/metaStore';
import { useRoute } from 'vue-router';
import { useCurrentNoteStore } from '../../stores/currentNoteStore';

const userConfig = useUserConfigStore();
const metaStore = useMetaStore();
const route = useRoute();
const currentNoteStore = useCurrentNoteStore();

const now = ref(Date.now());
const isSaving = ref(false);

let timeInterval: ReturnType<typeof setInterval> | null = null;
let unlistenSave: UnlistenFn | null = null;

onMounted(async () => {
  void userConfig.init();

  unlistenSave = await listen('note-saved', () => {
    isSaving.value = true;

    setTimeout(() => {
      isSaving.value = false;
    }, 1000);
  });

  timeInterval = setInterval(() => {
    now.value = Date.now();
  }, 1000);
});

onUnmounted(() => {
  if (timeInterval) {
    clearInterval(timeInterval);
    timeInterval = null;
  }

  if (unlistenSave) {
    unlistenSave();
    unlistenSave = null;
  }
});

const lastEdited = computed(() => {
  const updatedAt = currentNoteStore.currentNote?.updated_at;

  if (!updatedAt) {
    return 'Not edited yet';
  }

  return formatTimeAgo(updatedAt, now.value);
});

const currentLocation = computed(() => route.name);
const isEditor = computed(() => route.name === 'editor');

const encrypted = computed(() => userConfig.config['local.encryption']);
const local = computed(() => userConfig.config['online.sync'] === 'off');

const connected = computed<boolean | null>(
  () => metaStore.isConnectedToServer && metaStore.isConnectedToInternet
);

const isLocal = computed(() => userConfig.config['local.mode'] === 'off');

defineProps<{ version: string; synced: string }>();

function formatTimeAgo(timestamp: number, currentTime: number): string {
  const milliseconds = Math.max(0, currentTime - timestamp);

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
</script>

<template>
  <div
    class="flex h-7 w-full select-none flex-row items-center justify-between border-t border-white/5 bg-black/40 px-4 text-xs"
  >
    <div
      v-if="isEditor"
      class="flex items-center gap-3 text-note-pumice"
    >
      <span>{{ lastEdited }}</span>

      <div class="h-3 w-px bg-white/10" />

      <span>
        {{ currentNoteStore.words }}
        {{ currentNoteStore.words == 1 ? "word" : "words" }}
      </span>

      <div class="h-3 w-px bg-white/10" />

      <span>Markdown</span>

      <!-- save -->
      <div class="h-3 w-px bg-white/10" />

      <div
        class="flex items-center gap-1.5 transition-all duration-300"
        :class="isSaving ? 'text-note-glow' : 'text-note-pumice/40'"
      >
        <Save
          :size="16"
          :stroke-width="2"
          
          class="transition-transform duration-300 text-note-ivory/75"
          :class="isSaving ? 'scale-110' : 'scale-100'"
        />

        <span
          class="transition-all duration-300"
          :class="isSaving ? 'opacity-100' : 'w-0 overflow-hidden opacity-0'"
        >
          Saved
        </span>
      </div>
    </div>

    <div
      v-else
      class="flex items-center gap-3 text-note-pumice"
    >
      <span>Llava / {{ currentLocation }}</span>
    </div>

    <div class="flex items-center gap-2">
      <!-- online -->
      <div
        class="flex items-center gap-1.5 text-green-500"
        v-if="connected"
      >
        <Cloud :size="12" />
        <span>Online</span>
      </div>

      <!-- offline -->
      <div
        class="flex items-center gap-1.5 text-note-garnet"
        v-else
      >
        <CloudOff :size="12" />
        <span>Offline</span>
      </div>

      <div class="h-3 w-px bg-white/10" />

      <!-- sync off -->
      <div
        class="flex items-center gap-1.5 text-note-garnet"
        v-if="local"
      >
        <RefreshCwOff :size="12" />
        <span>Sync off</span>
      </div>

      <!-- synced -->
      <div
        class="flex items-center gap-1.5 text-note-glow"
        v-if="!local"
      >
        <CloudCheck :size="12" />
        <span>Sync on</span>
      </div>

      <div class="h-3 w-px bg-white/10" />

      <!-- encrypted -->
      <div
        class="flex items-center gap-1 rounded bg-note-glow/10 px-1.5 py-0.5 text-note-glow"
        v-if="encrypted == 'on'"
      >
        <Lock :size="11" />
        <span>Encrypted</span>
      </div>

      <!-- unencrypted -->
      <div
        class="flex items-center gap-1 rounded bg-note-garnet/10 px-1.5 py-0.5 text-note-garnet"
        v-else
      >
        <LockOpen :size="11" />
        <span>Unencrypted</span>
      </div>

      <div class="h-3 w-px bg-white/10" />

      <!-- local -->
      <div
        class="flex items-center gap-1 rounded bg-white/5 px-1.5 py-0.5 text-note-pumice"
        v-if="!isLocal"
      >
        <HardDrive :size="11" />
        <span>Disk</span>
      </div>

      <!-- cloud -->
      <div
        class="flex items-center gap-1 rounded bg-note-paprika/10 px-1.5 py-0.5 text-note-paprika"
        v-else
      >
        <Server :size="11" />
        <span>Cloud</span>
      </div>

      <div class="h-3 w-px bg-white/10" />

      <span class="text-note-pumice/30">
        v{{ version }}
      </span>
    </div>
  </div>
</template>