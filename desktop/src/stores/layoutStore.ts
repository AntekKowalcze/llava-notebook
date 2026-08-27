import { defineStore } from 'pinia';
import { ref } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { useToast } from 'vue-toastification';
const toast = useToast()
export const useLayoutStore = defineStore('layout', () => {

  const leftPanelOpen = ref<boolean>(false);
  const isTagEditorOpen = ref(false);
  const reencrypting = ref(false)
  let reencryptingStatusListener: (() => void) | null = null
let reencryptingFinishedListener: (() => void) | null = null

async function setupReencryptingListener() {
  if (reencryptingStatusListener || reencryptingFinishedListener) return

  reencryptingStatusListener = await listen("reencrypting_db", () => {
    reencrypting.value = true

    toast.warning(
      "Do not close the application or turn off your computer until re-encryption is complete."
    )
  })

  reencryptingFinishedListener = await listen("reencrypting_db_finished", () => {
    reencrypting.value = false

    toast.success("Operation completed successfully.")
    toast.success("You got new recovery codes in your clipboard, for you local password, old codes are not valid. Save them in encrypted places. ")
  })
}
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
    setupReencryptingListener,
    reencrypting,
    leftPanelOpen,
    toggleLeftPanel,
    isTagEditorOpen,
    openTagEditor,
    closeTagEditor,
  };
});
