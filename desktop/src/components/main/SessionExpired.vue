<script setup lang="ts">
import { ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useToast } from 'vue-toastification';
import { storeToRefs } from 'pinia';
import TextInput from '../auth/forms/TextInput.vue';
import { InputTypes } from '../../types/inputTypes';
import { X } from 'lucide-vue-next';
import LoadingCircle from '../main/LoadingCircle.vue';
import SubmitButton from '../commons/SubmitButton.vue';
import { useOnlineAuthStore } from '../../stores/onlineAuth';
import { useAuthStore } from '../../stores/auth';
import { computed } from 'vue';
import { useUserConfigStore } from '../../stores/userConfig';
//if was account linked just set task which try to log in when internet connection is back
const toast = useToast();
const onlineAuthStore = useOnlineAuthStore();
const authStore = useAuthStore();
const userConfig = useUserConfigStore();
const { settingList } = storeToRefs(userConfig);
const email = ref('');
const password = ref('');
const loading = ref<boolean>(false);
const emailPattern =
  /[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*@(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?/ ;

const isDisabled = computed(()=> {
  return !emailPattern.test(email.value);
})
//await window.__TAURI__.event.emit('online_session_expired', null) then implement timeuot handling on login, and implememnt logging online, after loging with code, and add register to settings then implement no internet, no server connection handling and add error hadling to this
function clearFields() {
  email.value = '';
  password.value = '';
}

function getErrorText(err: unknown): string {
  if (typeof err === 'string') return err;

  if (err && typeof err === 'object') {
    const typedErr = err as { message?: unknown; error?: unknown; reason?: unknown };
    if (typeof typedErr.message === 'string') return typedErr.message;
    if (typeof typedErr.error === 'string') return typedErr.error;
    if (typeof typedErr.reason === 'string') return typedErr.reason;
  }

  return String(err ?? '');
}

async function submit() {
  if (!email.value || !password.value) return;
  loading.value = true;
  try {
    if (!settingList.value) {
      await userConfig.init();
    }
    if (!settingList.value) {
      toast.error('Settings not ready yet');
      return;
    }
    const onlineUserId = await invoke<string>('login_online', {
      email: email.value,
      password: password.value,
      currentSettings: settingList.value,
    });
    toast.success('Connected accounts successfully');
    onlineAuthStore.$patch({
      loggedIn: true,
      loggedInId: onlineUserId,
    });
    authStore.$patch({
      linked: true,
    });
    await onlineAuthStore.fetchEmail();
    onlineAuthStore.setSessionExpired(false);
    clearFields();
  } catch (err: unknown) {
    console.log(err)
    // TODO online login is not implemented in full spec because of key manipulatino
    const message = getErrorText(err).toLowerCase();
    if (message.includes('wrong password')) {
      toast.warning('Wrong Password');
    } else if (message.includes('wrong credentials') || message.includes('invalid_credentials')) {
      toast.warning('Wrong email or password');
    } else if (message.includes('user does not exist') || message.includes('user not exists')) {
      toast.warning('User does not exist');
    } else if (message.includes('no internet connection')) {
      toast.error('No internet connection');
    } else if (message.includes('server not responding')) {
      toast.error('Server not responding');
    } else {
      toast.error('Login failed');
    }
  } finally {
    loading.value = false;
  }
}
//add regex email validation here
function cancel() {
  onlineAuthStore.setSessionExpired(false);
  clearFields();
}

watch(
  () => onlineAuthStore.sessionExpired,
  (isOpen) => {
    if (!isOpen) {
      clearFields();
    }
  }
);
</script>

<template>
  <div
    v-if="onlineAuthStore.sessionExpired"
    class="fixed inset-0 z-50 flex items-center justify-center"
  >
    <div
      class="absolute inset-0"
      @click="cancel"
    ></div>

    <div
      class="relative z-10 w-[90vw] max-w-md rounded-lg border border-note-pumice/20 bg-black/80 p-6 text-note-ivory shadow-lg"
    >
      <div class="mb-4 flex items-center justify-between">
        <h3 class="text-lg font-semibold">Session expired - log in again</h3>
        <X
          @click="cancel"
          class="h-10 w-10 cursor-pointer p-2 text-note-pumice"
        />
      </div>

      <TextInput
        :placeholder="'Email'"
        :type="InputTypes.Email"
        :name="'email'"
        v-model="email"
      />

      <TextInput
        class="mt-3"
        :placeholder="'Password'"
        :type="InputTypes.Password"
        :name="'password'"
        v-model="password"
      />

      <div class="mt-4 flex items-center justify-end gap-3">
        <button
          @click="cancel"
          class="h-10 rounded-md border border-note-pumice/20 bg-black/30 px-4 text-sm text-note-pumice/80 hover:bg-black/40"
        >
          Cancel
        </button>
        <SubmitButton
          v-if="!loading"
          :disabled="isDisabled"
          class="!mt-0 !h-10 !py-0 px-6"
          :content="'Log in'"
          @click="submit"
        />
        <LoadingCircle
          v-if="loading"
          style="transform: scale(0.4); transform-origin: center"
        />
      </div>
    </div>
  </div>
</template>

  <!--  TODO  SECOND TASK add tags manipulation -->
