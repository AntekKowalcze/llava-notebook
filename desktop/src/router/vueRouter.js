import { createWebHashHistory, createRouter } from "vue-router";
import { useAuthStore } from "../stores/auth";
import { invoke } from "@tauri-apps/api/core";
const routes = [
    {
        path: '/', name: "index",
        // redirect: "/chooseRegisterForm"
        beforeEnter: async () => {
            try {
                const authStore = useAuthStore()
                await authStore.checkUsers();
                const hasAnyUsers = authStore.hasAnyUsers
                console.log(hasAnyUsers)
                if (hasAnyUsers) {//first run
                    return { path: "/register", replace: true }
                } else if (hasAnyUsers && authStore.loggedIn) {
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
    { path: '/loading', name: "loading", component: () => import('../views/LoadingPage.vue') }
]

export const router = createRouter({
    history: createWebHashHistory(),
    routes,
    scrollBehavior(to, from, savedPosition) {
        return { top: 0 }
    }
})
//TODO add router guard so you cant move between login and not login sites
//TODO opóźnienie po iluś próbach, kody odzyskania hasła po rejestracji
//TODO potem dashboard 
// RZECZY PO LOGOWANIU REJESTEACJI -> change current user, change paths, 
// and then delete this temporary users dir, create user folder in users folder with subdirs

// change active  user -> get paths -> update paths in state => check if folders align with user names if not delete folders where there is no user aligned