import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { ref } from 'vue';
import { useOnlineAuthStore } from './onlineAuth';
import { useToast } from 'vue-toastification';

function getConnectionErrorToast(err: unknown): string | null {
  if (typeof err === 'string') {
    if (err === 'NoInternetConnection') {
      return 'No internet connection';
    }
    if (err === 'ServerNotAvailable') {
      return 'Server unavailable. Try again later.';
    }
    return null;
  }

  if (err && typeof err === 'object') {
    const typedErr = err as {
      NoInternetConnection?: unknown;
      ServerNotAvailable?: unknown;
    };

    if (typedErr.NoInternetConnection) {
      return 'No internet connection';
    }
    if (typedErr.ServerNotAvailable) {
      return 'Server unavailable. Try again later.';
    }
  }

  return null;
}

function toastStartupConnectionIssue(err: unknown) {
  const toast = useToast();
  const message = getConnectionErrorToast(err);
  if (message) {
    toast.error(message);
  }
}

export const useAuthStore = defineStore('auth', () => {
  const hasNoUsers = ref<boolean | null>(null);
  const loggedIn = ref(false);
  const loggedInUsername = ref<string | null>(null);
  const loggedInUserId = ref<string | null>(null);
  const recoveryKeys = ref<string[] | null>(null);
  const pendingCode = ref<string | null>(null);
  const linked = ref<boolean>(false);
  const sessionReady = ref(false);
  const onlineStatus = ref<'unknown' | 'checking' | 'logged_in' | 'not_logged_in' | 'not_linked'>(
    'unknown'
  );
  let sessionInit: Promise<void> | null = null;
  let onlineStatusListener: (() => void) | null = null;
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
        onlineStatus.value = 'logged_in';
        linked.value = true;
        onlineAuthStore.$patch({
          loggedIn: true,
          loggedInId: loggedInOnline.data ?? null,
        });
        await onlineAuthStore.fetchEmail();
      } else if (loggedInOnline.status === 'not_linked') {
        onlineStatus.value = 'not_linked';
        linked.value = false;
        onlineAuthStore.$patch({
          loggedIn: false,
          loggedInId: null,
        });
      } else if (loggedInOnline.status === 'not_logged_in') {
        onlineStatus.value = 'not_logged_in';
        linked.value = true;
        onlineAuthStore.$patch({
          loggedIn: false,
          loggedInId: null,
        });
        toastStartupConnectionIssue(loggedInOnline.data);
      } else if (loggedInOnline.status === 'checking') {
        onlineStatus.value = 'checking';
        linked.value = true;
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

  async function setupOnlineStatusListener() {
    if (onlineStatusListener) {
      return;
    }
    onlineStatusListener = await listen('online_login_status', async (event) => {
      const onlineAuthStore = useOnlineAuthStore();
      const toast = useToast();
      const payload = event.payload as { status: string; data?: any };
      console.log(payload.status + "   is logged in ?")
      if (payload.status === 'logged_in') {
        onlineStatus.value = 'logged_in';
        linked.value = true;
        onlineAuthStore.$patch({
          loggedIn: true,
          loggedInId: payload.data ?? null,
        });
        await onlineAuthStore.fetchEmail();
        if (onlineAuthStore.loggedInEmail) {
          toast.success(`logged in to online account: ${onlineAuthStore.loggedInEmail}`);
        } else {
          toast.success('logged in to online account');
        }
      } else if (payload.status === 'not_linked') {
        onlineStatus.value = 'not_linked';
        linked.value = false;
        onlineAuthStore.$patch({
          loggedIn: false,
          loggedInId: null,
        });
      } else if (payload.status === 'not_logged_in') {
        onlineStatus.value = 'not_logged_in';
        linked.value = true;
        onlineAuthStore.$patch({
          loggedIn: false,
          loggedInId: null,
        });
        const connectionError = getConnectionErrorToast(payload.data);
        if (connectionError) {
          toast.error(connectionError);
        } else {
          toast.warning('online user is not logged in');
        }
      }
    });
  }
  async function ensureSession() {
    if (sessionReady.value) {
      return;
    }
    if (!sessionInit) {
      sessionInit = (async () => {
        await setupOnlineStatusListener();
        await Promise.all([checkUsers(), checkSession()]);
        sessionReady.value = true;
      })();
    }
    await sessionInit;
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
    ensureSession,
    sessionReady,
    onlineStatus,
  };
});


