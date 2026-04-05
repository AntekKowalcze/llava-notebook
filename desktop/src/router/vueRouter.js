import { createWebHashHistory, createRouter } from 'vue-router';
import { useAuthStore } from '../stores/auth';
import { invoke } from '@tauri-apps/api/core';
import { useToast } from 'vue-toastification';
import MainView from '../views/MainView.vue';
const toast = useToast();
const routes = [
  {
    path: '/',
    name: 'index',

    beforeEnter: async () => {
      const authStore = useAuthStore();
      try {
        await Promise.all([authStore.checkUsers(), authStore.checkSession()]); //check session should probably return user id and then

        const hasNoUsers = authStore.hasNoUsers;
        if (hasNoUsers) {
          //first run
          return { path: '/register', replace: true };
        } else if (!hasNoUsers && authStore.loggedIn) {
          toast.success(authStore.loggedInUsername + ' logged in');
          return { path: '/main/', replace: true };
        } else if (!hasNoUsers && !authStore.loggedIn) {
          console.log('not logged in');
          return { path: '/login', replace: true };
        }
      } catch (err) {
        console.error(err);
      }
    },
    meta: { skipAuth: true },
  },
  {
    path: '/main',
    name: 'main',

    component: () => import('../views/MainView.vue'),
    children: [
      { path: '', name: 'editor', component: () => import('../views/LoadingPage.vue') }, // /main
      {
        path: 'dashboard',
        name: 'dashboard',
        component: () => import('../views/DashboardView.vue'),
      },
      { path: 'settings', name: 'settings', component: () => import('../views/SettingsView.vue') }, // /main/settings
    ],
  },
  {
    path: '/chooseRegisterForm',
    name: 'choose',
    component: () => import('../views/auth/RegisterAskPage.vue'),
  },
  {
    path: '/register',
    name: 'register',
    component: () => import('../views/auth/RegisterPage.vue'),
    meta: { skipAuth: true },
  },
  {
    path: '/login',
    name: 'login',
    component: () => import('../views/auth/LoginPage.vue'),
    meta: { skipAuth: true },
  },
  { path: '/loading', name: 'loading', component: () => import('../views/LoadingPage.vue') },
  {
    path: '/recoveryCodes',
    name: 'recoveryCodes',
    component: () => import('../views/auth/RecoveryCodesPage.vue'),
  },
  {
    path: '/changePassword',
    name: 'changePassword',
    component: () => import('../views/auth/ChangePassword.vue'),
  },
  {
    path: '/recovery',
    name: 'recovery',
    component: () => import('../views/auth/RecoveryPage.vue'),
    meta: { skipAuth: true },
  },
  {
    path: '/register/online',
    name: 'registerOnline',
    component: () => import('../views/auth/OnlineRegister.vue'),
    meta: {skipAuth: true}
  }
];
export const router = createRouter({
  history: createWebHashHistory(),
  routes,
  scrollBehavior(to, from, savedPosition) {
    return { top: 0 };
  },
});

router.beforeEach(async (to, from) => {
  if (to.matched.some((record) => record.meta && record.meta.skipAuth)) {
    return true;
  }
  const authStore = useAuthStore();
  try {
    await Promise.all([authStore.checkUsers(), authStore.checkSession()]);
  } catch (err) {
    console.error('auth checks failed', err);
    toast.error('Authentication check failed');
    return { path: '/login', replace: true };
  }

  if (authStore.hasNoUsers) {
    return { path: '/register', replace: true };
  }

  if (!authStore.loggedIn) {
    return { path: '/login', replace: true };
  }

  return true;
});
