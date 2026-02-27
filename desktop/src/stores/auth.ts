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
        try {
            const state = await invoke<{ status: string; user_id?: string }>('check_login_on_start');

            if (state.status === 'logged_in') {
                loggedInUserId.value = state.user_id ?? null;

                try {
                    loggedInUsername.value = await invoke<string>(
                        'get_username_from_uuid',
                        { userUuid: loggedInUserId.value }
                    );
                } catch (err) {
                    console.error('Failed to get username:', err);
                    loggedInUsername.value = null;
                }

                // ustawiasz loggedIn dopiero gdy masz już wszystkie dane
                loggedIn.value = true;
                console.log(loggedIn, loggedInUserId, loggedInUsername)
            } else {
                loggedIn.value = false;
            }
        } catch {
            loggedIn.value = false;
        }
    }



    return { hasNoUsers, loggedIn, loggedInUsername, loggedInUserId, recoveryKeys, checkUsers, pendingCode, checkSession };
});
