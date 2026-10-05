<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { invoke } from '@tauri-apps/api/core';
import ShortcutsHelp from './components/main/ShortcutsHelp.vue';
import { useShortcuts } from './lib/shortcuts';
import { errorKey } from './lib/errors';
import { useUserConfigStore } from './stores/userConfig';
import { useAuthStore } from './stores/auth';
import LoadingCircle from './components/main/LoadingCircle.vue';
import SessionExpired from './components/main/SessionExpired.vue';
import TitleBar from './components/TitleBar/TitleBar.vue';
import { useLayoutStore } from './stores/layoutStore.ts';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useToast } from 'vue-toastification';
const toast = useToast();

let unlisten: UnlistenFn | null = null;
let shown = false;

const authStore = useAuthStore();
const route = useRoute();
const router = useRouter();
const showShortcuts = ref(false);
const userConfig = useUserConfigStore();
const showSessionLoader = computed(() => !authStore.sessionReady);
const layoutStore = useLayoutStore();

// Lets the toast container (style.css) clear the editor's note header bar.
watch(
  () => route.name,
  (name) => {
    document.body.classList.toggle('editor-open', name === 'editor');
    // The tag editor flag is global; never carry an open editor to another page.
    layoutStore.closeTagEditor();
  },
  { immediate: true }
);

// App-wide shortcuts only apply inside the app, not on the login screens.
function inApp(): boolean {
  return route.path.startsWith('/main');
}

async function syncNow() {
  if (userConfig.config['local.mode'] === 'on') {
    toast.info('Turn off local mode in settings to sync notes');
    return;
  }
  if (userConfig.config['online.sync'] === 'off') {
    toast.info('Sync is turned off in settings');
    return;
  }

  try {
    // Progress and errors also show in the bottom bar's sync status.
    await invoke<void>('synchronize_all');
  } catch (err) {
    console.error('Manual sync failed:', err);
    switch (errorKey(err)) {
      case 'NotLoggedIn':
        toast.warning('Connect an online account to sync notes');
        break;
      case 'NoInternetConnection':
      case 'ServerNotAvailable':
        toast.warning('Cannot reach the server. Sync will retry automatically.');
        break;
      default:
        toast.warning('Sync failed. It will retry automatically.');
    }
  }
}

useShortcuts({
  toggleSidebar: () => inApp() && layoutStore.toggleLeftPanel(),
  newNote: () => inApp() && router.push({ name: 'create' }),
  // The notes list focuses its search box when this query value changes.
  quickOpen: () => inApp() && router.push({ name: 'allNotes', query: { search: Date.now() } }),
  settings: () => inApp() && router.push({ name: 'settings' }),
  dashboard: () => inApp() && router.push({ name: 'dashboard' }),
  syncNow: () => inApp() && void syncNow(),
  help: () => (showShortcuts.value = !showShortcuts.value),
});

onMounted(async () => {
  void authStore.ensureSession();
  unlisten = await listen('quota_exceeded', () => {
    if (shown) return;

    shown = true;

    toast.error('Your storage quota has been exceeded. New uploads are currently unavailable.', {
      timeout: 20000,
    });
  });
});

onUnmounted(() => {
  unlisten?.();
});
</script>
<template>
  <div class="flex h-screen w-full flex-col overflow-hidden bg-note-graphite bg-cover bg-center">
    <TitleBar class="shrink-0"></TitleBar>

    <session-expired></session-expired>

    <ShortcutsHelp
      v-if="showShortcuts"
      @close="showShortcuts = false"
    />

    <div
      v-if="showSessionLoader"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
    >
      <div class="flex flex-col items-center gap-4">
        <LoadingCircle />
      </div>
    </div>

    <main class="relative flex min-h-0 w-full flex-1 flex-col">
      <router-view />
    </main>
  </div>
</template>
