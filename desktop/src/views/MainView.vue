<script setup lang="ts">
import LeftSlidingPanel from '../components/LeftSlidingPanel.vue';
import BottomBar from '../components/main/BottomBar.vue';
import { useLayoutStore } from '../stores/layoutStore.ts';
const layout = useLayoutStore();
</script>

<template>
  <div class="flex h-full min-h-0 flex-col overflow-hidden">
    <div class="flex min-h-0 flex-1">
      <Transition
        enter-active-class="transition-all duration-300 ease-out"
        enter-from-class="-translate-x-full opacity-0"
        enter-to-class="translate-x-0 opacity-100"
        leave-active-class="transition-all duration-300 ease-in"
        leave-from-class="translate-x-0 opacity-100"
        leave-to-class="-translate-x-full opacity-0"
      >
        <LeftSlidingPanel v-if="layout.leftPanelOpen" />
      </Transition>
      <main class="min-h-0 flex-1 overflow-hidden">
        <RouterView />
      </main>
    </div>

    <BottomBar
      :version="'0.1.0'"
      :synced="'synced'"
      class="shrink-0"
    />
  </div>
</template>

<style scoped></style>
