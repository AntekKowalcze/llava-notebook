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
import { useOnlineAuthStore } from '../../stores/onlineAuth'
const onlineAuthStore = useOnlineAuthStore();;
const router = useRouter();
const email = ref<string>('');
const password = ref<string>('');
const repeatPassword = ref<string>('');
const isPasswordValid = ref<boolean>(false);
const toast = useToast();

const loading = ref(false);
const emailPattern = /[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*@(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?/g
const correctEmail = computed(() => {
    return email.value.match(emailPattern)
})
const passwordsMatch = computed(() => {
    return password.value === repeatPassword.value;
});


const canSubmit = computed(() => {
    return (
        isPasswordValid.value &&
        passwordsMatch.value &&
        repeatPassword.value.length > 0 &&
        email.value.length > 0 && passwordsMatch.value

    );
});

async function submitRegister() {
    if (!canSubmit.value) return;
    loading.value = true;

    try {
        await invoke<void>('register_user_online', {
            email: email.value,
            password: password.value,
            passwordRepeated: repeatPassword.value,
        });

        console.log('Registration success');
        onlineAuthStore.$patch({
            loggedIn: true,
            loggedInEmail: email.value,
        });
        toast.success('successfully regisered online user account');
        await router.replace("/main/");
    } catch (err: any) {
        toast.error(err)
        // if (err === 'UsernameExistsError') {
        //     toast.warning('Username already exists');
        // } else {
        //     toast.error('Failed registering local user');
        //     console.error('Registration failed:', err);
        // } //TODO add error matching here for better UX
    } finally {
        loading.value = false;
    }
}
</script>
<template>
    <FormCard header-text="Register" sub-text="create online account" class="pb-4">
        <template v-if="!loading">
            <TextInput :placeholder="'email'" :type="InputTypes.Email" :name="'email'" class="mt-6" v-model="email">
            </TextInput>
            <TextInput v-model:isValid="isPasswordValid" :placeholder="'password'" :type="InputTypes.Password"
                :name="'password'" v-model="password" show-validation></TextInput>
            <TextInput :placeholder="'repeat password'" :type="InputTypes.Password" :name="'repeatPassword'"
                v-model="repeatPassword"></TextInput>
            <TinyError v-if="repeatPassword && !passwordsMatch" error-content="Passwords do not match!"></TinyError>
            <TinyError v-if="!correctEmail && email.length > 0" error-content="This email is not correct"
                class="mt-2" />
            <SubmitButton :disabled="!canSubmit" :content="'Submit'" @click="submitRegister">
            </SubmitButton>
            <RouterLink to="/login/online" class="mb-0 mt-8 text-note-ivory/80 hover:underline">
                Do you have online account already? Login.
            </RouterLink>
        </template>
        <LoadingCircle v-else></LoadingCircle>
    </FormCard>
</template>
