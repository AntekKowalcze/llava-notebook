<script setup lang="ts">
import LeftSlidingPanel from '../components/LeftSlidingPanel.vue';
import BottomBar from '../components/main/BottomBar.vue';
import { useLayoutStore } from '../stores/layoutStore.ts';
const layout = useLayoutStore();
</script>
<template>
  <div class="flex h-full w-full flex-col overflow-hidden">
    <div class="relative flex min-h-0 w-full flex-1 overflow-hidden">
      <Transition
        enter-active-class="transition-all duration-300 ease-out"
        enter-from-class="-translate-x-full opacity-0"
        enter-to-class="translate-x-0 opacity-100"
        leave-active-class="transition-all duration-300 ease-in"
        leave-from-class="translate-x-0 opacity-100"
        leave-to-class="-translate-x-full opacity-0"
      >
        <LeftSlidingPanel
          v-if="layout.leftPanelOpen"
          class="z-40 shrink-0"
        />
      </Transition>

      <main class="relative flex h-full min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
        <RouterView class="h-full w-full flex-1" />
      </main>
    </div>

    <BottomBar
      :version="'1.0.2'"
      :synced="'synced'"
      class="w-full shrink-0"
    />
  </div>
</template>
