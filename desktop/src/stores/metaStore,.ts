import { defineStore } from 'pinia';
import { ref, watch } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { useToast } from 'vue-toastification';
import { invoke } from '@tauri-apps/api/core';
import { useOnlineAuthStore } from './onlineAuth';
export const useMetaStore = defineStore('metaStore', () => {
   let isConnectedToServer= ref<boolean | null>(null)
   let isConnectedToInternet = ref<boolean|null>(null)
    let serverStatusInitialized = false;
    let internetStatusInitialized = false;
   
    listen<boolean>("internet_connection_status", (status)=> {
        isConnectedToInternet.value = status.payload
    })
     listen<boolean>("server_connection_status", (status)=> {
        isConnectedToServer.value = status.payload
    })

    watch(isConnectedToServer, async (newValue, oldValue) => {
    if (newValue === null) return //
    if (!serverStatusInitialized) {
        serverStatusInitialized = true;
        return;
    }
    
    const toast = useToast()
    if (newValue && !oldValue) {
        toast.success("Connected to the server")
        let onlineAuthStore = useOnlineAuthStore()
        if (!onlineAuthStore.loggedIn) {
            try{
            await invoke<void>("try_login_if_connected_with_server")
            }catch (err){
                toast.warning("Not logged in to online account, try again in settings")
            }
        }
    } else if (!newValue && (oldValue === null || oldValue)) {  
        toast.warning("Lost connection to the server")
    }
}, { immediate: true })

    watch(isConnectedToInternet, (newValue, oldValue) => {
        if (newValue === null) return 
        if (!internetStatusInitialized) {
                internetStatusInitialized = true;
                return;
        }
    
    const toast = useToast()
    if (newValue && !oldValue) {
        toast.success("Internet connected")
    } else if (!newValue && (oldValue === null || oldValue)) {
        toast.warning("Lost internet connection")
    }
}, { immediate: true })

   return {
    isConnectedToInternet,
    isConnectedToServer
    }   
})


