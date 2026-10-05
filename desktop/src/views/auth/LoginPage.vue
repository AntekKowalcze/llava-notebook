<script setup lang="ts">
import FormCard from '../../components/auth/forms/FormCard.vue';
import TextInput from '../../components/auth/forms/TextInput.vue';
import { InputTypes } from '../../types/inputTypes';
import { computed, onBeforeUnmount, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useRouter } from 'vue-router';
import { useToast } from 'vue-toastification';
import { useAuthStore } from '../../stores/auth';
import SubmitButton from '../../components/commons/SubmitButton.vue';
import { errorKey, errorPayload } from '../../lib/errors';

const toast = useToast();

const router = useRouter();
const password = ref<string>();
const username = ref<string>();
const loading = ref(false);
const lockoutUntil = ref<number | null>(null);
let lockoutTimer: ReturnType<typeof setTimeout> | null = null;

const submitDisabled = computed(() => {
  const isLocked = lockoutUntil.value !== null && lockoutUntil.value > Date.now();
  return !username.value || !password.value || loading.value || isLocked;
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

async function submitLogin() {
  const authStore = useAuthStore(); //here before login command check if user is timeouted
  if (submitDisabled.value) {
    return;
  }

  loading.value = true;
  try {
    let timeout: number = await invoke<number>('check_timeout_before_submit', {
      username: username.value,
    });
    if (timeout > 0) {
      applyLockout(timeout);
      showTimeout(timeout);
      return;
    }
    let userId = await invoke<string>('login_command', {
      username: username.value,
      password: password.value,
    });
    toast.success('Logged in successfully');
    authStore.$patch({
      loggedIn: true,
      loggedInUsername: username.value,
      loggedInUserId: userId,
    });

    router.replace('/main/');
  } catch (err) {
    switch (errorKey(err)) {
      case 'AccountLocked': {
        const timeout = errorPayload<number>(err) ?? 0;
        if (timeout > 0) {
          applyLockout(timeout);
          showTimeout(timeout);
        } else {
          toast.error('Too many failed attempts. Try again in a moment.');
        }
        break;
      }
      case 'WrongPassword':
        toast.warning('Wrong Password');
        break;
      case 'UserNotExists':
        toast.warning('User does not exist!');
        break;
      default:
        console.error('Login failed:', err);
        toast.error('Login failed. Try again.');
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
    timeout: lengthMs,
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
    sub-text="log in to existing local account"
  >
    <TextInput
      :name="'username'"
      :placeholder="'username'"
      :type="InputTypes.Text"
      v-model="username"
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
      :to="{ name: 'recovery', query: { origin: 'login' } }"
      class="mt-12 text-note-ivory/80 hover:underline"
    >
      Forgot password?
    </RouterLink>
    <RouterLink
      to="/register"
      class="mt-12 text-note-ivory/80 hover:underline"
    >
      Do you want to create account?
    </RouterLink>
  </FormCard>
</template>
