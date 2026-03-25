import { defineStore } from 'pinia';
import { ref } from 'vue';
import { listen } from '@tauri-apps/api/event';
// import { UserConfig } from "../types/settingTypes";
export const useUserConfigStore = defineStore('userConfig', () => {
  const config = ref<Record<string, string>>({});
  listen<Record<string, string>>('config-updated', (event) => {
    config.value = event.payload;
  });
  return { config };
});
