import { defineStore } from 'pinia';
import { ref, watch } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { useToast } from 'vue-toastification';
export const useMetaStore = defineStore('metaStore', () => {
   let isConnectedToServer= ref<boolean | null>(null)
   let isConnectedToInternet = ref<boolean|null>(null)
   
    listen<boolean>("internet_connection_status", (status)=> {
        isConnectedToInternet.value = status.payload
    })
     listen<boolean>("server_connection_status", (status)=> {
        isConnectedToServer.value = status.payload
    })

    watch(isConnectedToServer, (newValue, oldValue) => {
    if (newValue === null) return //
    
    const toast = useToast()
    if (newValue && !oldValue) {
        toast.success("Connected to the server")
    } else if (!newValue && (oldValue === null || oldValue)) {
        toast.warning("Lost connection to the server")
    }
}, { immediate: true })

  watch(isConnectedToInternet, (newValue, oldValue) => {
    if (newValue === null) return 
    
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


