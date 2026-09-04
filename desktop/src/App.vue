<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue';
import { useAuthStore } from './stores/auth';
import LoadingCircle from './components/main/LoadingCircle.vue';
import SessionExpired from './components/main/SessionExpired.vue';
import TitleBar from './components/TitleBar/TitleBar.vue';
import { useLayoutStore } from './stores/layoutStore.ts';
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useToast } from 'vue-toastification'
const toast = useToast()

let unlisten: UnlistenFn | null = null
let shown = false

const authStore = useAuthStore();
const showSessionLoader = computed(() => !authStore.sessionReady);
const layoutStore = useLayoutStore();

function handleKeyDown(event: KeyboardEvent) {
  if (event.ctrlKey && event.key.toLowerCase() === 'b') {
    event.preventDefault();

    layoutStore.$patch({
      leftPanelOpen: !layoutStore.leftPanelOpen,
    });
  }
}

onMounted(async () => {
  void authStore.ensureSession();
  window.addEventListener('keydown', handleKeyDown);
   unlisten = await listen('quota_exceeded', () => {
    if (shown) return

    shown = true

    toast.error(
      'Your storage quota has been exceeded. New uploads are currently unavailable.',
      {
        timeout: 20000,
      },
    )
  })
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeyDown);
   unlisten?.()
});

</script>
<template>
  <div class="flex h-full min-h-0 flex-col overflow-hidden bg-note-graphite bg-cover bg-center">
    <TitleBar class="shrink-0"></TitleBar>
    <session-expired></session-expired>
    <div
      v-if="showSessionLoader"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
    >
      <div class="flex flex-col items-center gap-4">
        <LoadingCircle />
      </div>
    </div>

    <div class="min-h-0 flex-1 overflow-hidden">
      <router-view />
    </div>
  </div>
</template>
