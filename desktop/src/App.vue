<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { useAuthStore } from './stores/auth';
import LoadingCircle from './components/main/LoadingCircle.vue';
import SessionExpired from './components/main/SessionExpired.vue';
import TitleBar from './components/TitleBar/TitleBar.vue';
const authStore = useAuthStore();
const showSessionLoader = computed(() => !authStore.sessionReady);

onMounted(() => {
  void authStore.ensureSession();
});



</script>
<template>

  <div class="flex min-h-dvh h-full flex-col bg-note-graphite bg-cover bg-center">
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
    
    <router-view />
  </div>
</template>
