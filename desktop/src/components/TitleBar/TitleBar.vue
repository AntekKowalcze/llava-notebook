<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { PanelLeft, Minus, Maximize2, Minimize2, X, UserRound } from 'lucide-vue-next';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useLayoutStore } from '../../stores/layoutStore';
import StatusBox from './StatusBox.vue';
import { useMetaStore } from '../../stores/metaStore.ts';
import { useAuthStore } from '../../stores/auth.ts';
import { useOnlineAuthStore } from '../../stores/onlineAuth.ts';

const layout = useLayoutStore();
const metaStore = useMetaStore();
const authStore = useAuthStore();
const onlineAuthStore = useOnlineAuthStore();

const email = computed(() => onlineAuthStore.loggedInEmail);
const username = computed(() => authStore.loggedInUsername);

const internetConnection = computed(() => metaStore.isConnectedToInternet);

const serverConnection = computed(() => metaStore.isConnectedToServer);

function showSidebar() {
  layout.toggleLeftPanel();
}

const win = getCurrentWindow();

const isMaximized = ref(false);
let unlisten: (() => void) | undefined;

onMounted(async () => {
  isMaximized.value = await win.isMaximized();

  unlisten = await win.listen('tauri://resize', async () => {
    isMaximized.value = await win.isMaximized();
  });
});

onUnmounted(() => {
  unlisten?.();
});

const minimize = async () => {
  await win.minimize();
};

const toggleMaximize = async () => {
  await win.toggleMaximize();
  isMaximized.value = await win.isMaximized();
};

const closeWin = async () => {
  await win.close();
};
</script>

<template>
  <div
    data-tauri-drag-region
    class="flex h-8 shrink-0 select-none items-center justify-evenly bg-black/10"
  >
    <div class="flex items-center gap-1 pl-1">
      <button
        class="flex h-8 w-8 items-center justify-center rounded"
        @click="showSidebar"
      >
        <PanelLeft
          :size="15"
          :stroke-width="3"
          class="text-note-ivory"
        />
      </button>

      <span class="pointer-events-none pl-0.5 font-medium tracking-wide">
        <span class="text-lg text-note-ivory/50">llava</span>
        <span class="text-lg text-note-pumice/30">note</span>
      </span>
    </div>

    <div
      data-tauri-drag-region
      class="flex h-full flex-1 items-center justify-center"
    >
      <div class="group relative flex h-8 w-8 cursor-pointer items-center justify-center">
        <UserRound
          class="h-6 w-6 text-note-ivory transition-colors duration-150 group-hover:text-note-paprika"
          :stroke-width="1.5"
        />

        <StatusBox
          v-if="username"
          :current-user="username"
          :online-account="email"
          :internet-connection="internetConnection"
          :server-connection="serverConnection"
          class="pointer-events-none absolute left-1/2 top-full z-50 mt-2 hidden -translate-x-1/2 border border-note-pumice/10 group-hover:flex"
        />
      </div>
    </div>

    <div class="flex">
      <button
        class="flex h-8 w-11 items-center justify-center hover:bg-black"
        @click="minimize"
      >
        <Minus
          :size="18"
          class="text-note-glow"
        />
      </button>

      <button
        class="flex h-8 w-11 items-center justify-center hover:bg-black"
        @click="toggleMaximize"
      >
        <Minimize2
          v-if="isMaximized"
          :size="18"
          class="text-note-paprika"
        />
        <Maximize2
          v-else
          :size="18"
          class="text-note-paprika"
        />
      </button>

      <button
        class="hover:bg-garnet flex h-8 w-11 items-center justify-center transition-colors duration-100 hover:bg-black"
        @click="closeWin"
      >
        <X
          :size="18"
          :stroke-width="3"
          class="text-note-garnet"
        />
      </button>
    </div>
  </div>
</template>
