<script setup lang="ts">
import FormCard from '../components/forms/FormCard.vue';
import FormButtons from '../components/forms/FormButtons.vue';
import TextInput from '../components/forms/TextInput.vue';
import { InputTypes } from '../types/inputTypes';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useRouter } from 'vue-router';
import { useToast } from 'vue-toastification';
import { useAuthStore } from '../stores/auth';

const toast = useToast();

const router = useRouter();
const password = ref<string>();
const username = ref<string>();
const disabled = ref<boolean>(false);
async function submitLogin() {
  const authStore = useAuthStore(); //here before login command check if user is timeouted
  try {
    let timeout: number = await invoke<number>('check_timeout_before_submit', {
      username: username.value,
    });
    console.log(timeout);
    if (timeout > 0) {
      console.log(timeout);
      showTimeout(timeout);
      return;
    }
    let userId = await invoke<string>('login_command', {
      username: username.value,
      password: password.value,
    });
    console.log(userId);
    toast.success('Logged in successfully');
    authStore.$patch({
      loggedIn: true,
      loggedInUsername: username.value,
      loggedInUserId: userId,
    });

    router.replace('/main/');
  } catch (err: any) {
    console.log(err);
    if (err == 'WrongPassword') {
      console.log(err);
      toast.warning('Wrong Password', {});
    } else if (err.AccountLocked) {
      showTimeout(err.AccountLocked);
    } else if (err === 'UserNotExists') {
      toast.warning('User does not exist!');
    }
    return;
  }
}
function showTimeout(lengthMs: number) {
  const totalSeconds = Math.floor(lengthMs / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const secs = totalSeconds % 60;

  toast.error(`🔒Account locked for ${minutes}m ${String(secs).padStart(2, '0')}s`, {
    timeout: lengthMs,
  });
}
</script>
<template>
  <FormCard header-text="Sign in" sub-text="log in to existing offline account">
    <TextInput :name="'username'" :placeholder="'username'" :type="InputTypes.Text" v-model="username"></TextInput>
    <TextInput :name="'password'" :placeholder="'password'" :type="InputTypes.Password" v-model="password"></TextInput>
    <FormButtons :disabled="disabled" :content="'Submit'" @click="submitLogin"></FormButtons>

    <RouterLink :to="{ name: 'recovery', query: { origin: 'login' } }" class="mt-12 text-note-ivory/80 hover:underline">
      Forgot password?
    </RouterLink>
    <RouterLink to="/register" class="mt-12 text-note-ivory/80 hover:underline">
      Do you want to create account?
    </RouterLink>
  </FormCard>
</template>
