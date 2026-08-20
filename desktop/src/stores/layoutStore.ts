import { defineStore } from 'pinia';
import { ref } from 'vue';

export const useLayoutStore = defineStore('layout', () => {
  const leftPanelOpen = ref<boolean>(false);
  const isTagEditorOpen = ref(false);

  function openTagEditor() {
    isTagEditorOpen.value = true;
  }

  function closeTagEditor() {
    isTagEditorOpen.value = false;
  }
  function toggleLeftPanel() {
    leftPanelOpen.value = !leftPanelOpen.value;
  }
  return {
    leftPanelOpen,
    toggleLeftPanel,
    isTagEditorOpen,
    openTagEditor,
    closeTagEditor,
  };
});
