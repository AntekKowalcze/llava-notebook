import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export const useOnlineAuthStore = defineStore('onlineAuth', () => {
  const loggedIn = ref<boolean>(false);
  const loggedInEmail = ref<string | null>(null);
  const loggedInId = ref<string | null>(null);

  async function fetchEmail() {
    if (!loggedInId.value) return;
    try {
      loggedInEmail.value = await invoke<string>('get_email_from_id', {
        onlineId: loggedInId.value,
      });
    } catch (err) {
      console.log('failed to get email from user ID');
    }
  }

  return {
    loggedIn,
    loggedInEmail,
    loggedInId,
    fetchEmail,
  };
});
