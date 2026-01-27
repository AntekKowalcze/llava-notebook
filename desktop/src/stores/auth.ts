import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { ref } from "vue";


export const useAuthStore = defineStore('auth', () => {
    const hasAnyUsers = ref<boolean | null>(null);
    const loggedIn = ref(false);
    const loggedInUsername = ref<string | null>(null);
    async function checkUsers() {
        try {
            const exists = await invoke<boolean>("check_if_user_exists");
            hasAnyUsers.value = exists;
        } catch (error) {
            console.error(error);
            hasAnyUsers.value = false;
        }
    }



    return { hasAnyUsers, loggedIn, loggedInUsername, checkUsers };
});
