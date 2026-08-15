import { defineStore } from 'pinia';
import { ref, computed, watch } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { useToast } from 'vue-toastification';
import { invoke } from '@tauri-apps/api/core';
import { useOnlineAuthStore } from './onlineAuth';
import { useAuthStore } from './auth';
let listenersRegistered = false;

export const useMetaStore = defineStore('metaStore', () => {
  const isConnectedToServer = ref<boolean | null>(null);
  const isConnectedToInternet = ref<boolean | null>(null);

  if (!listenersRegistered) {
    listenersRegistered = true;

    listen<boolean>("internet_connection_status", (status) => {
      if (isConnectedToInternet.value === status.payload) return;
      isConnectedToInternet.value = status.payload;
    });

    listen<boolean>("server_connection_status", (status) => {
      if (isConnectedToServer.value === status.payload) return;
      isConnectedToServer.value = status.payload;
    });

    invoke<[boolean, boolean]>("get_connection_status")
      .then(([server, internet]) => {
        if (isConnectedToServer.value === null) isConnectedToServer.value = server;
        if (isConnectedToInternet.value === null) isConnectedToInternet.value = internet;
      })
      .catch(() => {});
  }

  const connectionStatus = computed<'unknown' | 'offline' | 'server-unreachable' | 'online'>(() => {
    if (isConnectedToInternet.value === null || isConnectedToServer.value === null) return 'unknown';
    if (!isConnectedToInternet.value) return 'offline';
    if (!isConnectedToServer.value) return 'server-unreachable';
    return 'online';
  });

  let loginInFlight = false;

  watch(isConnectedToInternet, (newValue) => {
    if (newValue === null) return;
    const toast = useToast();
    if (newValue) {
      toast.success("Internet connected");
    } else {
      toast.error("Lost internet connection");
    }
  }, { immediate: true });

watch(isConnectedToServer, async (newValue) => {
  if (newValue === null) return;

  if (newValue) {
    useToast().success("Connected to the server");

    const authStore = useAuthStore();
    await authStore.ensureSession();
    await authStore.waitForOnlineCheckToSettle(); 

    const onlineAuthStore = useOnlineAuthStore();
    if (!onlineAuthStore.loggedIn && !loginInFlight) {
      loginInFlight = true;
      try {
        await invoke<void>("try_login_if_connected_with_server");
      } catch (err) {
        console.error('try_login_if_connected_with_server failed:', err);
      } finally {
        loginInFlight = false;
      }
    }
  }else{
    useToast().warning("No server connection")
  }
}, { immediate: true });

  return {
    isConnectedToInternet,
    isConnectedToServer,
    connectionStatus,
  };
});
