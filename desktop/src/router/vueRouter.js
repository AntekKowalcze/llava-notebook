import { createWebHashHistory, createRouter } from "vue-router";
import { useAuthStore } from "../stores/auth";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "vue-toastification";
const toast = useToast()
const routes = [
    {
        path: '/', name: "index",
        beforeEnter: async () => {
            const authStore = useAuthStore()
            try {
                await Promise.all([authStore.checkUsers(), authStore.checkSession()]) //check session should probably return user id and then 
                const hasNoUsers = authStore.hasNoUsers
                if (hasNoUsers) {//first run
                    return { path: "/register", replace: true }
                } else if (!hasNoUsers && authStore.loggedIn) {
                    toast.success(authStore.loggedInUsername + " logged in")
                    return { path: "/main", replace: true }
                } else if (!hasNoUsers && !authStore.loggedIn) {
                    console.log("not logged in")
                    return { path: "/login", replace: true }
                }
            } catch (err) {
                console.error(err)
            }
        }
    },
    { path: '/main', name: 'main', component: () => import('../views/LoadingPage.vue') },
    { path: '/chooseRegisterForm', name: 'choose', component: () => import('../views/RegisterAskPage.vue') },
    { path: '/register', name: 'register', component: () => import('../views/RegisterPage.vue') },
    { path: '/login', name: "login", component: () => import('../views/LoginPage.vue') },
    { path: '/loading', name: "loading", component: () => import('../views/LoadingPage.vue') },
    { path: '/recoveryCodes', name: 'recoveryCodes', component: () => import('../views/RecoveryCodesPage.vue') },
    { path: '/changePassword', name: 'changePassword', component: () => import('../views/ChangePassword.vue'), },
    { path: '/recovery', name: 'recovery', component: () => import('../views/RecoveryPage.vue') }

]

export const router = createRouter({
    history: createWebHashHistory(),
    routes,
    scrollBehavior(to, from, savedPosition) {
        return { top: 0 }
    }
})
