<script setup lang="ts">
import { ref } from 'vue';
import { computed } from 'vue';
import TextInput from '../components/forms/TextInput.vue';
import { InputTypes } from '../types/inputTypes';
import FormButtons from "../components/forms/FormButtons.vue"
import FormCard from '../components/forms/FormCard.vue';
import TinyError from '../components/forms/TinyError.vue';
import { invoke } from '@tauri-apps/api/core';
import { useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth';
import { RouterLink } from 'vue-router';
import { useToast } from 'vue-toastification';
import LoadingCircle from '../components/LoadingCircle.vue';
const authStore = useAuthStore();
const router = useRouter();
const username = ref<string>('');
const password = ref<string>('');
const repeatPassword = ref<string>('');
const isPasswordValid = ref<boolean>(false);
const toast = useToast();
const keys = ref<string[]>();
const loading = ref(false)

const passwordsMatch = computed(() => {
  return password.value === repeatPassword.value;
});
const isUsernameNotEmpty = computed(() => {
  return username.value.length > 0
})
const canSubmit = computed(() => {
  return isPasswordValid.value &&
    passwordsMatch.value &&
    repeatPassword.value.length > 0 &&
    username.value.length > 0
});

async function submitRegister() {
  if (!canSubmit.value) return
  loading.value = true

  try {
    const [recoveryKeys, userId] = await invoke<[string[], string]>("register_command", {
      username: username.value,
      password: password.value,
      passwordRepeated: repeatPassword.value
    })
    keys.value = recoveryKeys

    console.log("Registration success");
    authStore.$patch({
      loggedIn: true,
      loggedInUsername: username.value,
      loggedInUserId: userId,
      hasNoUsers: false,
      recoveryKeys: keys.value
    })
    console.log(authStore.loggedIn, authStore.loggedInUsername);
    toast.success("successfully regisered local user account");
    await router.replace({ name: "recoveryCodes" });

  } catch (err: any) {//TODO add error matching
    if (err === "UsernameExistsError") {
      toast.warning("Username already exists")

    } else {
      toast.error("Failed registering local user");
      console.error("Registration failed:", err);
    }

  } finally {
    loading.value = false
  }

}


</script>
<template>

  <FormCard header-text="Register" sub-text="create account" class="pb-4">
    <template v-if="!loading">
      <TextInput :placeholder="'username'" :type="InputTypes.Text" :name="'username'" class="mt-6" v-model="username">
      </TextInput>
      <TextInput v-model:isValid="isPasswordValid" :placeholder="'password'" :type="InputTypes.Password"
        :name="'password'" v-model="password" show-validation></TextInput>
      <TextInput :placeholder="'repeat password'" :type="InputTypes.Password" :name="'repeatPassword'"
        v-model="repeatPassword">
      </TextInput>
      <TinyError v-if="repeatPassword && !passwordsMatch" error-content="Passwords do not match!"></TinyError>
      <TinyError v-if="!isUsernameNotEmpty" error-content="Username to short!" class="mt-2"></TinyError>
      <FormButtons :disabled="!canSubmit" :content="'Submit'" @click="submitRegister"></FormButtons>
      <RouterLink to="/login" class="mt-8 mb-0 text-note-ivory/80 hover:underline">Do you have account already? Login.
      </RouterLink>
    </template>
    <LoadingCircle v-else></LoadingCircle>
  </FormCard>
</template>
