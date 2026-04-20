import { defineStore } from 'pinia';
import { ref } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { UserConfig } from '../types/settingTypes';
import { invoke } from '@tauri-apps/api/core';
export const useUserConfigStore = defineStore('userConfig', async () => {
  const [settingListData, isDefaultData] = await invoke<[UserConfig, boolean]>('get_config_data')
  const config = ref<Record<string, string>>({});
  const settingList = ref<UserConfig>(settingListData)
  const isDefault = ref<boolean>(isDefaultData)
  listen<Record<string, string>>('config-updated', (event) => {
    config.value = event.payload; 
  });
  return { config };
});
//get here another listener which gets key and value and then runs find and update with this values so its changed like in setting view
//config is writen on register, so its created 
//i think the best solution is 1. move settingsList to config store with update handling, and now i will be able to change config from backend emitting event and it will be automaticly upadted in settingsView file as well
