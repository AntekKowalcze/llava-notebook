<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { PanelLeft, Minus, Maximize2, Minimize2, X, UserRound } from 'lucide-vue-next'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useLayoutStore } from '../stores/layoutStore'
const layout = useLayoutStore()
function showSidebar(){
layout.toggleLeftPanel()
}

const win = getCurrentWindow()

const isMaximized = ref(false)
let unlisten: (() => void) | undefined

onMounted(async () => {
  isMaximized.value = await win.isMaximized()

  unlisten = await win.listen('tauri://resize', async () => {
    isMaximized.value = await win.isMaximized()
  })
})

onUnmounted(() => {
  unlisten?.()
})

const minimize = async () => {
  await win.minimize()
}

const toggleMaximize = async () => {
  await win.toggleMaximize()
  isMaximized.value = await win.isMaximized()
}

const closeWin = async () => {
  await win.close()
}
</script>

<template>
  <div data-tauri-drag-region class="flex h-8 shrink-0 select-none items-center justify-evenly bg-black/10">
    <div class="flex items-center gap-1 pl-1">
      <button class="flex h-8 w-8 items-center justify-center rounded " @click="showSidebar">
        <PanelLeft :size="15" :stroke-width="3" class="text-note-ivory" />
      </button>

      <span class="pointer-events-none pl-0.5 font-medium tracking-wide">
        <span class="text-lg text-note-ivory/50">llava</span>
        <span class="text-lg text-note-pumice/30"> note</span>
      </span>
    </div>

    <div data-tauri-drag-region class="flex flex-1 h-full items-center justify-center">
      <UserRound class="text-note-ivory cursor-pointer" :stroke-width="1"></UserRound>
    </div>

    <div class="flex">
      <button class="flex h-8 w-11 items-center justify-center hover:bg-black" @click="minimize">
        <Minus :size="18" class="text-note-glow" />
      </button>

      <button class="flex h-8 w-11 items-center justify-center hover:bg-black " @click="toggleMaximize">
        <Minimize2 v-if="isMaximized" :size="18" class="text-note-paprika" />
        <Maximize2 v-else :size="18" class="text-note-paprika" />
      </button>

      <button
        class="flex h-8 w-11 items-center justify-center transition-colors duration-100 hover:bg-garnet hover:bg-black"
        @click="closeWin">
        <X :size="18" :stroke-width="3" class="text-note-garnet" />
      </button>
    </div>
  </div>
</template>