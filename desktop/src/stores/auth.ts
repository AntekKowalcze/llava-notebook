import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { ref } from "vue";


export const useAuthStore = defineStore('auth', () => {
    const hasNoUsers = ref<boolean | null>(null);
    const loggedIn = ref(false);
    const loggedInUsername = ref<string | null>(null);
    const recoveryKeys = ref<string[] | null>(null)
    const pendingCode = ref<string | null>(null)
    async function checkUsers() {
        try {
            const exists = await invoke<boolean>("check_if_user_exists");
            hasNoUsers.value = exists;//if there is no users set true
        } catch (error) {
            hasNoUsers.value = false;
        }
    }



    return { hasNoUsers, loggedIn, loggedInUsername, recoveryKeys, checkUsers, pendingCode };
});
