import { createWebHashHistory, createRouter } from 'vue-router';
import { useAuthStore } from '../stores/auth';
import { invoke } from '@tauri-apps/api/core';
import { useToast } from 'vue-toastification';
import { useOnlineAuthStore } from '../stores/onlineAuth';
import MainView from '../views/MainView.vue';
const toast = useToast();
const routes = [
  {
    path: '/',
    name: 'index',

    beforeEnter: async () => {
      const authStore = useAuthStore();
      const onlineAuthStore = useOnlineAuthStore();
      try {
        await authStore.ensureSession();

        const hasNoUsers = authStore.hasNoUsers;
        if (hasNoUsers) {
          //first run
          return { path: '/register', replace: true };
        } else if (!hasNoUsers && authStore.loggedIn) {
          toast.success(authStore.loggedInUsername + ' logged in');
          if (onlineAuthStore.loggedIn) {
            toast.success(onlineAuthStore.loggedInEmail + ' logged in to online account');
          } else {
            if (authStore.onlineStatus === 'not_logged_in') {
              toast.warning('failed to login to online account');
            }
          }
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
      {
        path: '',
        name: 'create',
        component: () => import('../views/NoteCreationView.vue'),
      },
      {
        path: 'editor/:noteId',
        name: 'editor',
        component: () => import('../views/editor/EditorView.vue'),
      },
      {
        path: 'dashboard',
        name: 'dashboard',
        component: () => import('../views/DashboardView.vue'),
      },
      {
        path: 'settings',
        name: 'settings',
        component: () => import('../views/SettingsView.vue'),
      },
      {
        path: 'allNotes',
        name: 'allNotes',
        component: () => import('../views/AllNotesView.vue'),
      },
      {
        path: 'removedNotes',
        name: 'removed',
        component: () => import('../views/RemovedNotes.vue'),
      },
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
    meta: { skipAuth: false },
    beforeEnter: (to, before, next) => {
      const onlineAuthStore = useOnlineAuthStore();
      if (onlineAuthStore.loggedIn) {
        toast.info('You are already logged in, logout first');
        next({ name: 'settings' });
      } else {
        next();
      }
    },
  },
  {
    path: '/login/online',
    name: 'loginOnline',
    component: () => import('../views/auth/OnlineLogin.vue'),
    meta: { skipAuth: false },
    beforeEnter: (to, before, next) => {
      const onlineAuthStore = useOnlineAuthStore();
      if (onlineAuthStore.loggedIn) {
        toast.info('You are already logged in, logout first');
        next({ name: 'settings' });
      } else {
        next();
      }
    },
  },
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
    await authStore.ensureSession();
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
