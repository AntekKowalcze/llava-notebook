<script setup lang="ts">
import FormCard from '../../components/auth/forms/FormCard.vue';
import SubmitButton from '../../components/commons/SubmitButton.vue';
import TextInput from '../../components/auth/forms/TextInput.vue';
import { InputTypes } from '../../types/inputTypes';
import { ref } from 'vue';
import { computed } from 'vue';
import { useRouter } from 'vue-router';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '../../stores/auth';

import { useToast } from 'vue-toastification';
import TinyError from '../../components/auth/forms/TinyError.vue';
const authStore = useAuthStore();
const username = authStore.loggedInUsername;
const router = useRouter();
const password = ref<string>('');
const repeatPassword = ref<string>('');
const isPasswordValid = ref<boolean>(false);
const toast = useToast();
const passwordsMatch = computed(() => {
  return password.value === repeatPassword.value;
});

const canSubmit = computed(() => {
  return isPasswordValid.value && passwordsMatch.value && repeatPassword.value.length > 0;
});
async function changePassword() {
  try {
    await invoke<void>('change_password', {
      username,
      password: password.value,
      passwordRepeated: repeatPassword.value,
      code: authStore.pendingCode,
    });
    authStore.$patch({ pendingCode: null });
    toast.success('Password changed sucessfully');
    router.replace({ name: 'loading' });
  } catch (err) {
    console.log(err);
    toast.error('error while chaning password');
  }
}
</script>

<template>
  <FormCard
    header-text="Change password"
    sub-text=""
  >
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
    <SubmitButton
      :disabled="!canSubmit"
      :content="'Submit'"
      @click="changePassword"
    ></SubmitButton>
    <RouterLink
      to="/loading"
      class="mb-0 mt-8 text-note-ivory/80 hover:underline"
    >
      Go to main
    </RouterLink>
  </FormCard>
</template>
