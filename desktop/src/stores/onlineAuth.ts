import { defineStore } from 'pinia';
import { ref } from 'vue';


export const useOnlineAuthStore = defineStore('onlineAuth', () => {
const loggedIn = ref<boolean>(false)
const loggedInEmail = ref<string | null>(null)
const loggedInId = ref<string|null>(null)


  return {
    loggedIn,
    loggedInEmail,
    loggedInId
  };
});
