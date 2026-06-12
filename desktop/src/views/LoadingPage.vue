<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import LoadingCircle from '../components/main/LoadingCircle.vue';

import IconComponent from '../components/main/IconComponent.vue';
import SubmitButton from '../components/commons/SubmitButton.vue';
import { useAuthStore } from '../stores/auth';
import { useToast } from 'vue-toastification';
import { useRouter } from 'vue-router';
import { useOnlineAuthStore } from '../stores/onlineAuth';
import { useUserConfigStore } from '../stores/userConfig';
import { UserRoundCheck, UserRoundX } from 'lucide-vue-next';
const router = useRouter();
const authStore = useAuthStore();
const onlineAuthStore = useOnlineAuthStore();
const userConfigStore = useUserConfigStore();
const toast = useToast();
let buttonContent = 'logout';
async function logout() {
  try {
    await invoke<void>('local_logout_command');
    authStore.$patch({
      loggedIn: false,
      loggedInUsername: null,
      loggedInUserId: null,
    });
        userConfigStore.settingList = null;

    onlineAuthStore.$patch({
      loggedIn: false,
      loggedInEmail: null,
      loggedInId: null
    })

    toast.success('logged out successfully');
    router.replace('/');
  } catch (err) {
    toast.error('Error while logggin out');
  }
}
</script>

<template>
  <IconComponent
    :height="'44'"
    :width="'44'"
  ></IconComponent>
  <LoadingCircle />
  <!-- LOGOUT -->
  <SubmitButton
    :disabled="false"
    :content="buttonContent"
    @click="logout"
  ></SubmitButton>
  <SubmitButton
    :disabled="false"
    :content="'go to settings'"
    @click="
      () => {
        router.replace('/main/settings');
      }
    "
  ></SubmitButton>
  <SubmitButton
    :disabled="false"
    :content="'go to dashboard'"
    @click="
      () => {
        router.replace('/main/dashboard');
      }
    "
  ></SubmitButton>
  <UserRoundCheck class="text-note-ivory" v-if="onlineAuthStore.loggedIn"></UserRoundCheck>
  <UserRoundX class="text-note-ivory" v-else></UserRoundX>
</template>
