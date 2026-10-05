<script setup lang="ts">
import { ref } from 'vue';
import { computed } from 'vue';
import TextInput from '../../components/auth/forms/TextInput.vue';
import { InputTypes } from '../../types/inputTypes';
import SubmitButton from '../../components/commons/SubmitButton.vue';
import FormCard from '../../components/auth/forms/FormCard.vue';
import TinyError from '../../components/auth/forms/TinyError.vue';
import { invoke } from '@tauri-apps/api/core';
import { useRouter } from 'vue-router';
import { RouterLink } from 'vue-router';
import { useToast } from 'vue-toastification';
import LoadingCircle from '../../components/main/LoadingCircle.vue';
import { useOnlineAuthStore } from '../../stores/onlineAuth';
import { useUserConfigStore } from '../../stores/userConfig';
import { errorKey } from '../../lib/errors';
import { isValidEmail } from '../../lib/validation';
const onlineAuthStore = useOnlineAuthStore();
const userConfig = useUserConfigStore();
const router = useRouter();
const email = ref<string>('');
const password = ref<string>('');
const repeatPassword = ref<string>('');
const isPasswordValid = ref<boolean>(false);
const toast = useToast();

const loading = ref(false);
const correctEmail = computed(() => isValidEmail(email.value));
const passwordsMatch = computed(() => {
  return password.value === repeatPassword.value;
});

const canSubmit = computed(() => {
  return (
    isPasswordValid.value &&
    passwordsMatch.value &&
    repeatPassword.value.length > 0 &&
    correctEmail.value &&
    email.value.length > 0 &&
    passwordsMatch.value
  );
});
async function submitRegister() {
  if (!canSubmit.value) return;
  loading.value = true;

  let previousMode: string | null = null;
  try {
    await userConfig.init();
    if (!userConfig.settingList) {
      toast.error('Settings not loaded. Try again.');
      return;
    }
    previousMode = userConfig.getValueBySettingId(userConfig.settingList.sections, 'local.mode');
    userConfig.updateSettingValue('local.mode', 'off');
    await invoke<void>('register_user_online', {
      email: email.value,
      password: password.value,
      passwordRepeated: repeatPassword.value,
      currentSettings: userConfig.settingList,
    });

    onlineAuthStore.$patch({
      loggedIn: true,
      loggedInEmail: email.value,
    });
    toast.success('successfully regisered and connected to online account');
    await router.replace('/main/');
  } catch (err) {
    console.error('Online registration failed:', err);
    // Put back what was there instead of assuming local mode was on.
    if (previousMode !== null && previousMode !== 'ID NOT EXISTS') {
      userConfig.updateSettingValue('local.mode', previousMode);
    }

    switch (errorKey(err)) {
      case 'NoInternetConnection':
        toast.error('No internet connection');
        break;
      case 'ServerNotAvailable':
        toast.error('Server unavailable. Try again later.');
        break;
      case 'EmailAlreadyUsed':
        toast.warning('Email already used');
        break;
      case 'WrongEmail':
        toast.warning('Invalid email address');
        break;
      case 'PasswordValidation':
        toast.warning('Password does not meet requirements');
        break;
      case 'RequestError':
        toast.error('Server error. Try again later.');
        break;
      case 'InternalError':
        toast.error('Registration failed. Try again later.');
        break;
      default:
        toast.error('Failed to register user, try again');
    }
  } finally {
    loading.value = false;
  }
}
</script>
<template>
  <FormCard
    header-text="Register"
    sub-text="create online account"
    class="pb-4"
  >
    <template v-if="!loading">
      <TextInput
        :placeholder="'email'"
        :type="InputTypes.Email"
        :name="'email'"
        class="mt-6"
        v-model="email"
      ></TextInput>
      <TextInput
        v-model:isValid="isPasswordValid"
        :placeholder="'password'"
        :type="InputTypes.Password"
        :name="'password'"
        v-model="password"
        show-validation
      ></TextInput>
      <TextInput
        :placeholder="'repeat password'"
        :type="InputTypes.Password"
        :name="'repeatPassword'"
        v-model="repeatPassword"
      ></TextInput>
      <TinyError
        v-if="repeatPassword && !passwordsMatch"
        error-content="Passwords do not match!"
      ></TinyError>
      <TinyError
        v-if="!correctEmail && email.length > 0"
        error-content="This email is not correct"
        class="mt-2"
      />
      <SubmitButton
        :disabled="!canSubmit"
        :content="'Submit'"
        @click="submitRegister"
      ></SubmitButton>

      <RouterLink
        to="/login/online"
        class="mb-0 mt-8 text-note-ivory/80 hover:underline"
      >
        Do you have online account already? Login.
      </RouterLink>

      <RouterLink
        to="/main/"
        class="mb-0 mt-4 text-note-ivory/80 hover:underline"
      >
        Cancel
      </RouterLink>
    </template>
    <LoadingCircle v-else></LoadingCircle>
  </FormCard>
</template>
