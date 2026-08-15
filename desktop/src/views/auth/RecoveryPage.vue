<script setup lang="ts">
import SubmitButton from '../../components/commons/SubmitButton.vue';
import FormCard from '../../components/auth/forms/FormCard.vue';
import TextInput from '../../components/auth/forms/TextInput.vue';
import { InputTypes } from '../../types/inputTypes';
import { ref } from 'vue';
import { useAuthStore } from '../../stores/auth';
import { useRoute } from 'vue-router';
import { invoke } from '@tauri-apps/api/core';
import { useToast } from 'vue-toastification';
import { useRouter } from 'vue-router';
import LoadingCircle from '../../components/main/LoadingCircle.vue';
const toast = useToast();

const route = useRoute();
const router = useRouter();
let code = ref<string>('');
let username = ref<string>('');
const loading = ref<boolean>(false)
const authStore = useAuthStore();
const originRaw = (route.query.origin as string | undefined) ?? 'login';
// normalize: allow values like "settings", "/settings", "login" or "/login"
const originKey = originRaw.replace(/^\/+/, '');
const originTo =
  originKey === 'settings'
    ? '/main/settings'
    : originKey === 'login'
      ? '/login'
      : originRaw.startsWith('/')
        ? originRaw
        : `/${originKey}`;
async function checkCode() {
  try {
    loading.value= true
    let [userId, one_code] = await invoke<[string, boolean]>('log_with_code', {
      username: username.value,
      code: code.value,
    });
    if (one_code) {
      toast.info('You have used all of your codes, generate more in settings');
    }
    authStore.$patch({
      loggedIn: true,
      loggedInUsername: username.value,
      loggedInUserId: userId,
      pendingCode: code.value,
    });
    console.log(authStore.pendingCode)
    toast.success('Code correct, logged in successfully');
    router.replace({ path: '/changePassword' });
    loading.value = false
  } catch (err: any) {
    loading.value = false
    console.log(err);
    const errorKey =
      typeof err === 'string'
        ? err
        : typeof err?.error === 'string'
          ? err.error
          : typeof err?.message === 'string'
            ? err.message
            : null;
    if (errorKey === 'WrongPassword' || errorKey === 'CodeNotFound' || err?.WrongPassword || err?.CodeNotFound) {
      toast.warning('Code does not exist');
    } else if (errorKey === 'UserNotExists' || err?.UserNotExists) {
      toast.warning('User does not exist');
    } else if (errorKey === 'NoInternetConnection' || err?.NoInternetConnection) {
      toast.error('No internet connection');
    } else if (errorKey === 'ServerNotAvailable' || err?.ServerNotAvailable) {
      toast.error('Server unavailable. Try again later.');
    } else if (errorKey === 'OnlineSessionExpired' || err?.OnlineSessionExpired) {
      toast.warning('Online session expired. Try again.');
    } else if (errorKey === 'RequestError' || err?.RequestError) {
      toast.error('Server error. Try again later.');
    } else if (errorKey === 'LockError' || err?.LockError) {
      toast.error('App is busy. Try again.');
    } else if (
      errorKey === 'FileOperationError' ||
      errorKey === 'InternalError' ||
      err?.FileOperationError ||
      err?.InternalError
    ) {
      toast.error('Internal error. Try again.');
    } else {
      toast.error('internal error');
    }
  }
}
</script>

<template>
  <FormCard header-text="Enter recovery key" sub-text="enter the recovery code you received when logging in">
    <template v-if="!loading">
    <TextInput name="username" placeholder="enter username" :type="InputTypes.Text" v-model="username"></TextInput>
    <TextInput name="code" placeholder="enter recovery code" :type="InputTypes.Text" class="mb-24 mt-20" v-model="code">
    </TextInput>

    <SubmitButton content="submit" @click="checkCode"></SubmitButton>
    </template>
    <LoadingCircle v-else></LoadingCircle>
    <RouterLink :to="originTo" class="mb-0 mt-8 text-note-ivory/80 hover:underline">
      Go back
    </RouterLink>
  </FormCard>
  
</template>
