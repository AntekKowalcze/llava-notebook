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
  RefreshCw,
  RefreshCwOff,
  CloudCheck,
  Save,
  CloudAlert,
  CloudSync,
} from 'lucide-vue-next';
import { useMetaStore } from '../../stores/metaStore';
import { useRoute } from 'vue-router';
import { useCurrentNoteStore } from '../../stores/currentNoteStore';
import { useOnlineAuthStore } from '../../stores/onlineAuth';
const syncResult = computed(() => metaStore.syncResult);
const userConfig = useUserConfigStore();
const metaStore = useMetaStore();
const route = useRoute();
const currentNoteStore = useCurrentNoteStore();
const onlineAuthStore = useOnlineAuthStore();
const now = ref(Date.now());
const isSaving = ref(false);

let timeInterval: ReturnType<typeof setInterval> | null = null;
let unlistenSave: UnlistenFn | null = null;
const isLoggedInOnline = computed(() => {
  return onlineAuthStore.loggedIn;
});

onMounted(async () => {
  void userConfig.init();

  unlistenSave = await listen('note-saved', async () => {
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
const local = computed(() => {
  return userConfig.config['local.mode'] == 'on';
});
const syncPossible = computed(
  () => userConfig.config['online.sync'] === 'on' && isLoggedInOnline.value
);
const syncEnabled = computed(() => {
  return userConfig.config['online.sync'] === 'on';
});

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
    class="flex h-7 w-full shrink-0 select-none flex-row items-center justify-between overflow-x-auto whitespace-nowrap border-t border-white/5 bg-black/40 px-2 text-[10px] [-ms-overflow-style:none] [scrollbar-width:none] sm:px-4 sm:text-xs [&::-webkit-scrollbar]:hidden"
  >
    <div
      v-if="isEditor"
      class="flex items-center gap-2 text-note-pumice sm:gap-3"
    >
      <span>{{ currentNoteStore.words }}</span>
      <span>{{ currentNoteStore.words == 1 ? 'word' : 'words' }}</span>

      <div class="h-3 w-px bg-white/10" />

      <span>{{ lastEdited }}</span>

      <div class="h-3 w-px bg-white/10" />

      <span>Markdown</span>

      <div class="h-3 w-px bg-white/10" />

      <div
        class="flex items-center gap-1 transition-all duration-300 sm:gap-1.5"
        :class="isSaving ? 'text-note-glow' : 'text-note-pumice/40'"
      >
        <Save
          :size="14"
          :stroke-width="2"
          class="text-note-ivory/75 transition-transform duration-300 sm:h-[16px] sm:w-[16px]"
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
      class="flex items-center gap-2 text-note-pumice sm:gap-3"
    >
      <span>Llava / {{ currentLocation }}</span>
    </div>

    <!-- Right Side Settings/Statuses -->
    <div class="ml-4 flex items-center gap-1.5 sm:gap-2">
      <!-- sync result -->
      <div
        v-if="!local && syncPossible"
        class="flex items-center gap-1 sm:gap-1.5"
      >
        <div
          v-if="syncResult === 'InProgress'"
          class="flex items-center gap-1 text-note-paprika sm:gap-1.5"
        >
          <RefreshCw
            :size="12"
            class="animate-spin sm:h-[13px] sm:w-[13px]"
          />
          <span>Sync in progress</span>
        </div>

        <div
          v-else-if="syncResult === 'Done'"
          class="flex items-center gap-1 text-green-500 sm:gap-1.5"
        >
          <CloudCheck
            :size="12"
            class="sm:h-[13px] sm:w-[13px]"
          />
          <span>Synced</span>
        </div>

        <div
          v-else-if="syncResult === 'Error'"
          class="flex items-center gap-1 text-note-garnet sm:gap-1.5"
        >
          <CloudOff
            :size="12"
            class="sm:h-[13px] sm:w-[13px]"
          />
          <span>Error</span>
        </div>

        <div
          v-else-if="syncResult === 'NotSynced'"
          class="flex items-center gap-1 text-note-garnet sm:gap-1.5"
        >
          <CloudAlert
            :size="12"
            class="sm:h-[13px] sm:w-[13px]"
          />
          <span>Not synced yet</span>
        </div>
      </div>

      <div class="h-3 w-px bg-white/10" />

      <!-- sync off -->
      <div
        v-if="!syncEnabled"
        class="flex items-center gap-1 text-note-garnet sm:gap-1.5"
      >
        <RefreshCwOff
          :size="11"
          class="sm:h-[12px] sm:w-[12px]"
        />
        <span>Sync off</span>
      </div>

      <!-- sync on -->
      <div
        v-else
        class="flex items-center gap-1 text-green-500 sm:gap-1.5"
      >
        <CloudSync
          :size="11"
          class="sm:h-[12px] sm:w-[12px]"
        />
        <span>Sync on</span>
      </div>

      <div class="h-3 w-px bg-white/10" />

      <!-- encrypted -->
      <div
        v-if="encrypted == 'on'"
        class="flex items-center gap-1 rounded bg-note-glow/10 px-1.5 py-0.5 text-note-glow"
      >
        <Lock
          :size="10"
          class="sm:h-[11px] sm:w-[11px]"
        />
        <span>Encrypted</span>
      </div>

      <!-- unencrypted -->
      <div
        v-else
        class="flex items-center gap-1 rounded bg-note-garnet/10 px-1.5 py-0.5 text-note-garnet"
      >
        <LockOpen
          :size="10"
          class="sm:h-[11px] sm:w-[11px]"
        />
        <span>Unencrypted</span>
      </div>

      <div class="h-3 w-px bg-white/10" />

      <!-- local -->
      <div
        v-if="!isLocal"
        class="flex items-center gap-1 rounded bg-white/5 px-1.5 py-0.5 text-note-pumice"
      >
        <HardDrive
          :size="10"
          class="sm:h-[11px] sm:w-[11px]"
        />
        <span>Local mode</span>
      </div>

      <!-- cloud -->
      <div
        v-else
        class="flex items-center gap-1 rounded bg-note-paprika/10 px-1.5 py-0.5 text-note-paprika"
      >
        <Server
          :size="10"
          class="sm:h-[11px] sm:w-[11px]"
        />
        <span>Online mode</span>
      </div>

      <div class="h-3 w-px bg-white/10" />

      <span class="text-note-pumice/30">v{{ version }}</span>
    </div>
  </div>
</template>
