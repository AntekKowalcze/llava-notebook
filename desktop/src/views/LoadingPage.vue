<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import LoadingCircle from "../components/LoadingCircle.vue";

import IconComponent from "../components/IconComponent.vue";
//LOGOUT LOGIC
import FormButtons from "../components/forms/FormButtons.vue";
import { useAuthStore } from "../stores/auth";
import { useToast } from "vue-toastification";
import { useRouter } from "vue-router";
const router = useRouter();
const authStore = useAuthStore();
const toast = useToast();
let buttonContent = "logout"
async function logout() {
    try {
        await invoke<void>('local_logout_command', { userUuid: authStore.loggedInUserId })
        authStore.$patch({
            loggedIn: false,
            loggedInUsername: null,
            loggedInUserId: null
        })
        toast.success("logged out successfully")
        router.replace("/")
    } catch (err) {
        toast.error("Error while logggin out")
    }
}
</script>

<template>
    <IconComponent></IconComponent>
    <LoadingCircle />
    <!-- LOGOUT -->
    <FormButtons :disabled="false" :content="buttonContent" @click="logout"></FormButtons>
</template>