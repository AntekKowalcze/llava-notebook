import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import { ref } from 'vue';
import { useOnlineAuthStore } from './onlineAuth';

export const useAuthStore = defineStore('auth', () => {
  const hasNoUsers = ref<boolean | null>(null);
  const loggedIn = ref(false);
  const loggedInUsername = ref<string | null>(null);
  const loggedInUserId = ref<string | null>(null);
  const recoveryKeys = ref<string[] | null>(null);
  const pendingCode = ref<string | null>(null);
  const linked = ref<boolean>(false);
  async function checkUsers() {
    try {
      const exists = await invoke<boolean>('check_if_user_exists');
      hasNoUsers.value = exists; //if there is no users set true
    } catch (error) {
      hasNoUsers.value = false;
    }
  }
  async function checkSession() {
    const onlineAuthStore = useOnlineAuthStore();
    try {
      const [sessionState, loggedInOnline] =
        await invoke<
          [{ status: string; [key: string]: any }, { status: string; [key: string]: any }]
        >('check_login_on_start');
      console.log(loggedInOnline, sessionState); //why notes_key empty? also why online errors
      if (sessionState.status === 'logged_in') {
        loggedInUserId.value = sessionState.user_id ?? null;
        try {
          loggedInUsername.value = await invoke<string>('get_username_from_uuid', {
            userUuid: loggedInUserId.value,
          });
        } catch (err) {
          console.error('Failed to get username:', err);
          loggedInUsername.value = null;
        }

        loggedIn.value = true;
        console.log(loggedIn, loggedInUserId, loggedInUsername);
      } else {
        loggedIn.value = false;
      }
      if (loggedInOnline.status === 'logged_in') {
        linked.value = true;
        onlineAuthStore.$patch({
          loggedIn: true,
          loggedInId: loggedInOnline.data ?? null,
        });
        await onlineAuthStore.fetchEmail();
      } else if ((loggedInOnline.status = 'not_linked')) {
        linked.value = false;
        onlineAuthStore.$patch({
          loggedIn: false,
          loggedInId: null,
        });
      }
    } catch (err) {
      console.error('checkSession error:', err);
      loggedIn.value = false;
    }
  }

  return {
    hasNoUsers,
    loggedIn,
    loggedInUsername,
    loggedInUserId,
    recoveryKeys,
    checkUsers,
    pendingCode,
    linked,
    checkSession,
  };
});
