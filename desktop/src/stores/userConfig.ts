import { defineStore } from 'pinia';
import { ref } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { UserConfig } from '../types/settingTypes';
import { invoke } from '@tauri-apps/api/core';
import { Section } from '../types/settingTypes';

export const useUserConfigStore = defineStore('userConfig', () => {
  const config = ref<Record<string, string>>({});
  const settingList = ref<UserConfig | null>(null);
  const isDefault = ref<boolean>(false);
  let initPromise: Promise<void> | null = null;
  let listening = false;

  function updateSettingValue(id: string, value: string): boolean {
    if (!settingList.value) return false; // mayby it returns false here at start?
    return findAndUpdate(settingList.value.sections, id, value);
  }

  function applyConfigState(state: Record<string, string>) {
    config.value = state;
    for (const [id, value] of Object.entries(state)) {
      updateSettingValue(id, value);
    }
  }

  async function init() {
    if (initPromise) return initPromise;

    initPromise = (async () => {
      const [settingListData, isDefaultData] =
        await invoke<[UserConfig, boolean]>('get_config_data');
      settingList.value = settingListData;
      isDefault.value = isDefaultData;

      try {
        const configState = await invoke<Record<string, string>>('get_config_state');
        applyConfigState(configState);
      } catch (err) {
        console.warn('get_config_state failed:', err);
      }

      if (!listening) {
        listen<Record<string, string>>('config-updated', (event) => {
          applyConfigState(event.payload);
        });
        listening = true;
      }
    })().finally(() => {
      initPromise = null;
    });

    return initPromise;
  }

  function findAndUpdate(sections: Section[], id: string, value: string): boolean {
    for (const section of sections) {
      for (const setting of section.sectionSettings) {
        if (setting.id === id) {
          setting.currentValue = value;
          return true;
        }
      }
      if (section.subsections) {
        if (findAndUpdate(section.subsections, id, value)) return true;
      }
    }
    return false;
  }

  return { config, settingList, isDefault, updateSettingValue, init };
});