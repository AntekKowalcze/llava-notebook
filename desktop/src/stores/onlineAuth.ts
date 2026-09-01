import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
export const useOnlineAuthStore = defineStore('onlineAuth', () => {
  const loggedIn = ref<boolean>(false);
  const loggedInEmail = ref<string | null>(null);
  const loggedInId = ref<string | null>(null);
  const sessionExpired = ref<boolean>(false);
  async function fetchEmail() {
    if (!loggedInId.value) return;
    try {
      loggedInEmail.value = await invoke<string>('get_email_from_id', {
        onlineId: loggedInId.value,
      });
    } catch (err) {
    }
  }
  listen<void>('online_session_expired', () => {
    sessionExpired.value = true;
  });
  listen<string>('logged_in_online', async (event) => {
    loggedIn.value = true;
    loggedInId.value = event.payload;
    await fetchEmail();
  });

  function setSessionExpired(value: boolean) {
    sessionExpired.value = value;
  }

  return {
    loggedIn,
    loggedInEmail,
    loggedInId,
    sessionExpired,
    fetchEmail,
    setSessionExpired,
  };
});
