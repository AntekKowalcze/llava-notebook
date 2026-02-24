import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { ref } from "vue";


export const useAuthStore = defineStore('auth', () => {
    const hasNoUsers = ref<boolean | null>(null);
    const loggedIn = ref(false);
    const loggedInUsername = ref<string | null>(null);
    const loggedInUserId = ref<string | null>(null)
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
    async function checkSession() {
        console.log("here")
        try {
            const state = await invoke<{ status: string; user_id?: string }>('check_login_on_start');
            console.log(state.status)
            if (state.status === 'logged_in') {

                loggedIn.value = true;
                loggedInUserId.value = state.user_id ?? null;
                console.log(loggedIn.value, loggedInUserId.value);

            } else {
                loggedIn.value = false;
            }
        } catch {
            loggedIn.value = false;
        }


    }


    return { hasNoUsers, loggedIn, loggedInUsername, loggedInUserId, recoveryKeys, checkUsers, pendingCode, checkSession };
});
//TODO add logout