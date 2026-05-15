<script setup lang="ts">
import FormCard from '../../components/auth/forms/FormCard.vue';
import TextInput from '../../components/auth/forms/TextInput.vue';
import { InputTypes } from '../../types/inputTypes';
import { computed, onBeforeUnmount, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useRouter } from 'vue-router';
import { useToast } from 'vue-toastification';
import SubmitButton from '../../components/commons/SubmitButton.vue';
import { useOnlineAuthStore } from '../../stores/onlineAuth';
import { useAuthStore } from '../../stores/auth';
import { useUserConfigStore } from '../../stores/userConfig';
const toast = useToast();
const authStore = useAuthStore()
const onlineAuthStore = useOnlineAuthStore()
const router = useRouter();
const password = ref<string>();
const email = ref<string>("");
const userConfig = useUserConfigStore();
const loading = ref(false);
const lockoutUntil = ref<number | null>(null);
let lockoutTimer: ReturnType<typeof setTimeout> | null = null;

const emailPattern =
  /[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*@(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?/ ;
const correctEmail = computed(() => {
  return emailPattern.test(email.value);
});

const submitDisabled = computed(() => {
  const isLocked = lockoutUntil.value !== null && lockoutUntil.value > Date.now();
  return !correctEmail.value || !password.value || loading.value || isLocked;
});

function applyLockout(timeoutMs: number) {
  if (timeoutMs <= 0) {
    return;
  }

  lockoutUntil.value = Date.now() + timeoutMs;

  if (lockoutTimer) {
    clearTimeout(lockoutTimer);
  }

  lockoutTimer = setTimeout(() => {
    lockoutUntil.value = null;
    lockoutTimer = null;
  }, timeoutMs);
}

function extractTimeout(err: unknown): number | null {
  if (err && typeof err === 'object') {
    const typedErr = err as { AccountLocked?: unknown };
    if (typeof typedErr.AccountLocked === 'number') {
      return typedErr.AccountLocked;
    }
  }
  return null;
}

async function submitLogin() {
  if (submitDisabled.value) {
    return;
  }

  loading.value = true;
  try {
    userConfig.updateSettingValue('local.mode', 'off');
    let online_user_id = await invoke<string>('login_online', {
      email: email.value,
      password: password.value,
      currentSettings: userConfig.settingList,
    });
    toast.success('Logged in to online account successfully');
    
    onlineAuthStore.$patch({
      loggedIn: true,
      loggedInId: online_user_id
    });
    authStore.$patch({
      linked: true
    })

    await onlineAuthStore.fetchEmail()
    router.replace('/main/');
  } catch (err: any) {
    userConfig.updateSettingValue('local.mode', 'on');
    const timeout = extractTimeout(err);
    if (timeout !== null) {
      applyLockout(timeout);
      showTimeout(timeout);
      return;
    }

    if (err?.NoInternetConnection) {
      toast.error('No internet connection');
      return;
    }

    if (err?.ServerNotAvailable) {
      toast.error('Server unavailable. Try again later.');
      return;
    }

    if (err?.WrongPassword) {
      toast.warning('Wrong password');
    } else if (err?.WrongCredentials) {
      toast.warning('Wrong email or password');
    } else if (err?.RequestError) {
      toast.error('Server error. Try again later.');
    } else {
      toast.error('Login failed');
    }
    return;
  } finally {
    loading.value = false;
  }
}

function showTimeout(lengthMs: number) {
  const totalSeconds = Math.floor(lengthMs / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const secs = totalSeconds % 60;

  toast.error(`🔒Account locked for ${minutes}m ${String(secs).padStart(2, '0')}s`, {
    timeout: lengthMs
  });
}

onBeforeUnmount(() => {
  if (lockoutTimer) {
    clearTimeout(lockoutTimer);
  }
});
</script>
<template>
  <FormCard
    header-text="Sign in"
    sub-text="log in to existing online account"
  >
    <TextInput
      :name="'email'"
      :placeholder="'email'"
      :type="InputTypes.Email"
      v-model="email"
    ></TextInput>
    <TextInput
      :name="'password'"
      :placeholder="'password'"
      :type="InputTypes.Password"
      v-model="password"
    ></TextInput>
    <SubmitButton
      :disabled="submitDisabled"
      :content="'Submit'"
      @click="submitLogin"
    ></SubmitButton>
 <RouterLink
      to="/register/online"
      class="mt-12 text-note-ivory/80 hover:underline" 

    >
      Do you want to create online account?
    </RouterLink>
    <RouterLink
      :to="'/main/settings'" 
      class="mt-4 text-note-ivory/80 hover:underline"
    >
      Return
    </RouterLink>
   
  </FormCard>
</template>
<!-- TODO check timeout changes both on frontend and on backend in files login.rs submit button loginPage onlinelogin , test it in application with local account and online account-->
 <!-- TODO check timeouts, and test them -->