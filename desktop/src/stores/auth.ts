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
  }
  return null;
}

type SessionStatusPayload = { status: string; [key: string]: any };

const NOT_LOGGED_IN_GRACE_MS = 1200;

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

  let notLoggedInToastShown = false;
  let pendingNotLoggedInTimeout: ReturnType<typeof setTimeout> | null = null;

  let onlineCheckSettleResolvers: Array<() => void> = [];

  function resolveOnlineCheckSettleWaiters() {
    const resolvers = onlineCheckSettleResolvers;
    onlineCheckSettleResolvers = [];
    resolvers.forEach((resolve) => resolve());
  }
  function waitForOnlineCheckToSettle(): Promise<void> {
    if (onlineStatus.value !== 'checking') {
      return Promise.resolve();
    }
    return new Promise((resolve) => {
      const timeoutId = setTimeout(() => {
        onlineCheckSettleResolvers = onlineCheckSettleResolvers.filter((r) => r !== wrapped);
        resolve();
      }, 5000);
      const wrapped = () => {
        clearTimeout(timeoutId);
        resolve();
      };
      onlineCheckSettleResolvers.push(wrapped);
    });
  }

  function cancelPendingNotLoggedInToast() {
    if (pendingNotLoggedInTimeout) {
      clearTimeout(pendingNotLoggedInTimeout);
      pendingNotLoggedInTimeout = null;
    }
  }

  function toastNotLoggedInIfNeeded(data: unknown) {
    if (notLoggedInToastShown || pendingNotLoggedInTimeout) return;

    const connectionError = getConnectionErrorToast(data);
    if (connectionError) {
      // Hard failure (no internet / server down) — no login attempt is
      // coming, so there's nothing worth waiting for. Toast immediately.
      notLoggedInToastShown = true;
      useToast().error(connectionError);
      return;
    }

    // Ambiguous case: could be a genuine "not logged in", or just a
    // transient status reported mid-attempt right before "logged_in"
    // arrives. Wait briefly — if login succeeds in that window, this
    // never fires.
    pendingNotLoggedInTimeout = setTimeout(() => {
      pendingNotLoggedInTimeout = null;
      notLoggedInToastShown = true;
      useOnlineAuthStore().loggedIn = false
      useToast().warning('online user is not logged in');
    }, NOT_LOGGED_IN_GRACE_MS);
  }

  function markLoggedIn() {
    cancelPendingNotLoggedInToast();
    notLoggedInToastShown = false;
  }

  async function checkUsers() {
    try {
      const exists = await invoke<boolean>('check_if_user_exists');
      hasNoUsers.value = exists;
    } catch (error) {
      hasNoUsers.value = false;
    }
  }

  async function checkSession() {
    const onlineAuthStore = useOnlineAuthStore();
    try {
      const [sessionState, loggedInOnline] =
        await invoke<[SessionStatusPayload, SessionStatusPayload]>('check_login_on_start');

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
      } else {
        loggedIn.value = false;
      }

      if (loggedInOnline.status === 'logged_in') {
        onlineStatus.value = 'logged_in';
        linked.value = true;
        markLoggedIn();
        onlineAuthStore.$patch({ loggedIn: true, loggedInId: loggedInOnline.data ?? null });
        await onlineAuthStore.fetchEmail();
        resolveOnlineCheckSettleWaiters();
      } else if (loggedInOnline.status === 'not_linked') {
        onlineStatus.value = 'not_linked';
        linked.value = false;
        onlineAuthStore.$patch({ loggedIn: false, loggedInId: null });
        resolveOnlineCheckSettleWaiters();
      } else if (loggedInOnline.status === 'not_logged_in') {
        onlineStatus.value = 'not_logged_in';
        linked.value = true;
        onlineAuthStore.$patch({ loggedIn: false, loggedInId: null });
        toastNotLoggedInIfNeeded(loggedInOnline.data);
        resolveOnlineCheckSettleWaiters();
      } else if (loggedInOnline.status === 'checking') {
        onlineStatus.value = 'checking';
        linked.value = true;
        onlineAuthStore.$patch({ loggedIn: false, loggedInId: null });
        // deliberately do NOT resolve settle-waiters here — still pending
      }
    } catch (err) {
      console.error('checkSession error:', err);
      loggedIn.value = false;
    }
  }

  async function setupOnlineStatusListener() {
    if (onlineStatusListener) return;
    onlineStatusListener = await listen('online_login_status', async (event) => {
      const onlineAuthStore = useOnlineAuthStore();
      const toast = useToast();
      const payload = event.payload as { status: string; data?: any };

      if (payload.status === 'logged_in') {
        onlineStatus.value = 'logged_in';
        linked.value = true;
        markLoggedIn();
        onlineAuthStore.$patch({ loggedIn: true, loggedInId: payload.data ?? null });
        await onlineAuthStore.fetchEmail();
        if (onlineAuthStore.loggedInEmail) {
          toast.success(`logged in to online account: ${onlineAuthStore.loggedInEmail}`);
        } else {
          toast.success('logged in to online account');
        }
        resolveOnlineCheckSettleWaiters();
      } else if (payload.status === 'not_linked') {
        onlineStatus.value = 'not_linked';
        linked.value = false;
        onlineAuthStore.$patch({ loggedIn: false, loggedInId: null });
        resolveOnlineCheckSettleWaiters();
      } else if (payload.status === 'not_logged_in') {
        onlineStatus.value = 'not_logged_in';
        linked.value = true;
        onlineAuthStore.$patch({ loggedIn: false, loggedInId: null });
        toastNotLoggedInIfNeeded(payload.data);
        resolveOnlineCheckSettleWaiters();
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
    waitForOnlineCheckToSettle,
  };
});
