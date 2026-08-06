import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useLayoutStore = defineStore('layout', () => {
    const leftPanelOpen = ref<boolean>(false)

    function toggleLeftPanel() {
        leftPanelOpen.value = !leftPanelOpen.value
    }
    return {
        leftPanelOpen,
        toggleLeftPanel
    }
})