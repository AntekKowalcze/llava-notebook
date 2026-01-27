<script setup lang="ts">
import FormCard from '../components/forms/FormCard.vue';
import FormButtons from '../components/forms/FormButtons.vue';
import TextInput from '../components/forms/TextInput.vue';
import { InputTypes } from '../types/inputTypes';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useRouter } from 'vue-router';
const router = useRouter()
const password = ref<string>()
const username = ref<string>()
async function submitLogin() {
    try {
        await invoke<void>('login_command', {
            username: username.value,
            password: password.value
        })
        router.replace({ name: "loading" })
    } catch (err) {
        console.log(err)
    }

}
</script>
<template>
    <FormCard header-text="Sign in" sub-text="log in to existing offline account">
        <TextInput :name="'username'" :placeholder="'username'" :type="InputTypes.Text" v-model="username"></TextInput>
        <TextInput :name="'password'" :placeholder="'password'" :type="InputTypes.Password" v-model="password">
        </TextInput>
        <FormButtons :disabled="false" :content="'Submit'" @click="submitLogin"></FormButtons>
        <RouterLink to="/register" class="mt-12 text-note-ivory/80 hover:underline">Do you want to create account?
        </RouterLink>
    </FormCard>
</template>
<!-- TODO add toasts for communicates like login failed -->