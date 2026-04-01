<script setup lang="ts">
import FormButtons from '../components/forms/FormButtons.vue';
import FormCard from '../components/forms/FormCard.vue';
import TextInput from '../components/forms/TextInput.vue';
import { InputTypes } from '../types/inputTypes';
import { ref } from 'vue';
import { useAuthStore } from '../stores/auth';
//logout not redirecting
import { useRoute } from 'vue-router';
import { invoke } from '@tauri-apps/api/core';
import { useToast } from 'vue-toastification';
import { useRouter } from 'vue-router';
const toast = useToast();
const route = useRoute();
const router = useRouter();
let code = ref<string>('');
let username = ref<string>('');
const authStore = useAuthStore();
const originRaw = (route.query.origin as string | undefined) ?? 'login';
// normalize: allow values like "settings", "/settings", "login" or "/login"
const originKey = originRaw.replace(/^\/+/, '');
const originTo = originKey === 'settings' ? '/main/settings' : originKey === 'login' ? '/login' : originRaw.startsWith('/') ? originRaw : `/${originKey}`;
async function checkCode() {
  try {
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
    toast.success('Code correct, logged in successfully');
    router.replace({ path: '/changePassword' });
  } catch (err: any) {
    console.log(err);
    if (err == 'WrongPassword') {
      toast.warning('Code does not exist');
    } else if (err === 'UserNotExist') {
      toast.warning('User does not exist');
    } else {
      toast.error('internal error');
    }
  }
}
</script>

<template>
  <FormCard header-text="Enter recovery key" sub-text="enter the recovery code you received when logging in">
    <TextInput name="username" placeholder="enter username" :type="InputTypes.Text" v-model="username"></TextInput>
    <TextInput name="code" placeholder="enter recovery code" :type="InputTypes.Text" class="mb-24 mt-20" v-model="code">
    </TextInput>
    <FormButtons content="submit" @click="checkCode"></FormButtons>
    <RouterLink :to="originTo" class="mb-0 mt-8 text-note-ivory/80 hover:underline">
      Go back
    </RouterLink>
  </FormCard>
</template>
