import { createWebHashHistory, createRouter } from "vue-router";
import { useAuthStore } from "../stores/auth";
import { invoke } from "@tauri-apps/api/core";
const routes = [
    {
        path: '/', name: "index",
        // redirect: "/chooseRegisterForm"
        beforeEnter: async () => {
            const authStore = useAuthStore()
            try {
                await authStore.checkUsers();
                const hasNoUsers = authStore.hasNoUsers
                console.log(hasNoUsers)
                if (hasNoUsers) {//first run
                    return { path: "/register", replace: true }
                } else if (!hasNoUsers && authStore.loggedIn) {
                    return { path: "/main", replace: true }
                } else {
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
    {
        path: '/changePassword', name: 'changePassword', component: () => import('../views/ChangePassword.vue'),
        //     beforeEnter: async () => {
        //         const authStore = useAuthStore()

        //         if (authStore.loggedIn) {
        //             console.log(authStore.loggedIn)
        //             return true;
        //         } else {
        //             return { path: "/recovery", replace: true }
        //         }
        //     },
    },
    { path: '/recovery', name: 'recovery', component: () => import('../views/RecoveryPage.vue') }

]

export const router = createRouter({
    history: createWebHashHistory(),
    routes,
    scrollBehavior(to, from, savedPosition) {
        return { top: 0 }
    }
})
//TODO add router guard so you cant move between login and not login sites
//TODO potem dashboard 