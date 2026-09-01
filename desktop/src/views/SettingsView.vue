<script setup lang="ts">
import { Funnel, InfoIcon, Search } from 'lucide-vue-next';
import { ArrowBigLeftDash } from 'lucide-vue-next';
import ScreenDeviderHorizontal from '../components/dashboard/ScreenDeviderHorizontal.vue';
import { useAuthStore } from '../stores/auth';
import { useRouter } from 'vue-router';
import { invoke } from '@tauri-apps/api/core';
import { onMounted, ref, watch } from 'vue';
import { Section, Setting, UserConfig } from '../types/settingTypes';
import SectionComp from '../components/settings/SectionComp.vue';
import { useToast } from 'vue-toastification';
import CheckboxInput from '../components/settings/CheckboxInput.vue';
import metaphone from '../lib/metaphone';
import LogPreview from '../components/settings/LogPreview.vue';
import PasswordInput from '../components/settings/PasswordInput.vue';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import ChangeUsername from '../components/settings/ChangeUsername.vue';
import { useUserConfigStore } from '../stores/userConfig';
import { storeToRefs } from 'pinia';
import { useOnlineAuthStore } from '../stores/onlineAuth';
import { emit } from '@tauri-apps/api/event';
const toast = useToast();
const authStore = useAuthStore();
const onlineAuthStore = useOnlineAuthStore();
const router = useRouter();
const userConfigStore = useUserConfigStore();
const { settingList, isDefault } = storeToRefs(userConfigStore);
const shouldSyncOnLogout = ref<boolean>(true)
const cardSettingsIdList: string[] = [
  'local.mode',
  'local.encryption',
  'local.logout',

  'local.showLogs',
  'local.deleteLocalFiles',
];
const cardSettings = ref<Setting[]>([]);
const username = ref<string | null>(authStore.loggedInUsername);
const newUsername = ref<string>('');
const id = authStore.loggedInUserId;
const showFilter = ref<boolean>(false);
let filters = ['local', 'local.core', 'local.danger', 'online', 'online.core', 'online.ai'];
const searchText = ref<string>('');
let settingsToShow: string[]; //list of setting ids got from metaphone
let previousSearchTextLength: number = 0;
let metaphoneCache: string[][] = [];
let metaphoneMap: Record<string, string[]>;
let showLogs = ref<boolean>(false);
const logContents = ref<string>('');
const showPasswordInput = ref<boolean>(false);
const showUsernameInput = ref<boolean>(false);
const passwordResolver = ref<((value: string | null) => void) | null>(null);
const usernameLoading = ref<boolean>(false);
const codesLoading = ref<boolean>(false);
function search() {
  if (settingList.value == null) return;
  if (searchText.value.length == 0) {
    initSettingVisibilityOnInputEnter(settingList.value.sections); //set all to false
    metaphoneCache = [];
  }
  if (searchText.value.length > 0 && searchText.value.length < previousSearchTextLength) {
    metaphoneCache.pop(); //if length was 5 and we went to 4 pop this 5 searech because there is sall change same letter will be written
    settingsToShow = metaphoneCache[searchText.value.length - 1];
    for (let settingId of settingsToShow) {
      changeSettingVisibility(settingList.value.sections, settingId);
    }
    return;
  }
  previousSearchTextLength = searchText.value.length;
  let processedString = metaphone(searchText.value);
  settingsToShow = metaphoneMap[processedString];
  if (settingsToShow == undefined) {
    settingsToShow = [];
  }
  for (let settingId of settingsToShow) {
    changeSettingVisibility(settingList.value.sections, settingId);
  }
  metaphoneCache.push(settingsToShow);

  return;
}

function initSettingVisibilityOnInputEnter(sections: Section[]) {
  for (let section of sections) {
    for (let setting of section.sectionSettings) {
      setting.show = false;
    }
    if (section.subsections) {
      initSettingVisibilityOnInputEnter(section.subsections);
    }
  }
}

function changeSettingVisibility(sections: Section[], settingId: string): boolean {
  for (const section of sections) {
    for (const setting of section.sectionSettings) {
      if (setting.id === settingId) {
        setting.show = true;
        return true;
      }
    }

    if (section.subsections) {
      if (changeSettingVisibility(section.subsections, settingId)) return true;
    }
  }

  return false;
}
function redirect() {
  router.replace('/main/');
}
function initVisibility(sections: Section[]) {
  for (const section of sections) {
    if (section.show === undefined) section.show = true;
    if (section.subsections) initVisibility(section.subsections);
  }
}
function initSettingVisibility(sections: Section[], state: boolean) {
  for (const section of sections) {
    for (const setting of section.sectionSettings) {
      setting.show = state;
    }
    if (section.subsections) initSettingVisibility(section.subsections, state);
  }
}
function rebuildCardSettings() {
  if (!settingList.value) {
    cardSettings.value = [];
    return;
  }

  const nextList: Setting[] = [];
  for (const settingId of cardSettingsIdList) {
    const found = findSetting(settingList.value.sections, settingId);
    if (found != null) nextList.push(found);
  }
  cardSettings.value = nextList;
}

watch(
  settingList,
  () => {
    rebuildCardSettings();
  },
  { deep: true, immediate: true }
);

onMounted(async () => {
  try {
    await userConfigStore.init();
    metaphoneMap = await invoke<Record<string, string[]>>('get_methapone_map');

    if (isDefault.value)
      toast.info(
        'Default config was written, you can restore last working version by restore option in settings'
      );
    if (settingList.value) {
      initSettingVisibility(settingList.value.sections, true);
      initVisibility(settingList.value.sections);
    } else {
      throw new Error('Failed to obtain user config');
    }
  } catch (e) {
    console.warn('get_config_data failed:', e);
    toast.error('failed to get current config');
  }
});

function showFilters() {
  showFilter.value = !showFilter.value;
}
async function handleChange(id: string, value: string) {
  if (!settingList.value) return;
  userConfigStore.updateSettingValue(id, value);
  try {
    if (id != 'local.loadConfigBackup' && id != 'local.logout') {
      await invoke('update_settings', { userConfig: settingList.value });
    }
  } catch (err) {
    toast.error('Failed to save config');
  }
  await handleUpdate(id);
}

async function handleUpdate(id: string) {
  
  switch (id) {
    case 'local.logout': {
      try {
        await invoke<void>('local_logout_command', {});
        authStore.$patch({
          loggedIn: false,
          loggedInUsername: null,
          loggedInUserId: null,
        });
        userConfigStore.settingList = null;
        onlineAuthStore.$patch({
          loggedIn: false,
          loggedInEmail: null,
          loggedInId: null,
        });

        toast.success('logged out successfully');
        router.replace('/');
      } catch (err) {
        toast.error('Error while logggin out');
      }
      break;
    }
    case 'local.loadConfigBackup': {
      try {
        await invoke<void>('load_backup_config');
        const [userConfig, _isDefault] = await invoke<[UserConfig, boolean]>('get_config_data');
        settingList.value = userConfig;
        initVisibility(settingList.value.sections);
        initSettingVisibility(settingList.value.sections, true);

        toast.success('backup loaded successfully');
      } catch (err) {
        
        toast.error('failed to load backup config');
      }
      break;
    }
    case 'local.showLogs': {
      try {
        logContents.value = await invoke<string>('get_logfile_content');
        showLogs.value = true;
      } catch (err) {
        
        toast.error('failed to show logs');
      }
      break;
    }
    case 'local.generateRecoveryCodes': {
      try {
        const pwd = await askForPassword();
        if (!pwd) return;
        codesLoading.value = true;
        const codes = await invoke<string[]>('get_recovery_codes', { password: pwd });
        codesLoading.value = false;
        passwordResolver.value?.(null);
        passwordResolver.value = null;
        let codeString = codes.join('\n');
        await writeText(codeString);
        toast.success(
          'Codes copied successfully, remember, never show codes to other people and store them in encrypted places',
          { timeout: 10000 }
        );
        codesLoading.value = false;
        showPasswordInput.value = false;
      } catch (err) {
        codesLoading.value = false;

        toast.error('failed to generate codes');
      }
      break;
    }
   
    case 'local.changePassword': {
      router.replace({ name: 'recovery', query: { origin: 'settings' } });
      break;
    }
    case 'online.logout': {
      try {
        await invoke<void>('online_logout', {sync: shouldSyncOnLogout.value});
        onlineAuthStore.$patch({
          loggedIn: false,
          loggedInEmail: null,
          loggedInId: null,
        });
        authStore.linked = false;
        toast.success('Disconnected online account from local account successfully. Notes synchronized with this account will be removed from this device. Local-only notes will remain.');
      } catch (err: any) {
        if (err?.NoInternetConnection) {
          toast.error('No internet connection. Try again later.');
        } else if (err?.RequestError) {
          toast.error('Server error. Try again later.');
        } else if (err?.ServerNotAvailable) {
          toast.error('Server unavailable. Try again later.');
        }else if (err?.SyncFailed){
          toast.error('Synchronization failed, some notes may be lost if you proceed to disconnect accounts, to disconnect, click disconnect again')
          shouldSyncOnLogout.value = false
          try {
          await invoke<void>('online_logout', {sync: shouldSyncOnLogout.value});
            toast.success('Disconnected online account from local account successfully. Notes synchronized with this account will be removed from this device. Local-only notes will remain.');
          }catch(err) {
            toast.error("Couldnt disconnect accounts, try again later")
          }
          shouldSyncOnLogout.value = true
        } else {
          toast.error('Logout failed');
        }
      }
      break;
    }
    case 'online.register': {
      router.replace({ name: 'registerOnline' });
      try {
      } catch (err) {}
      break;
    }
    case 'online.login': {
      router.replace({ name: 'loginOnline', query: { origin: 'settings' } });
      try {
      } catch (err) {}
      break;
    }
    case 'online.sync': {
      await emit("reload-left-panel")
      break;
    }
     case 'online.ai': {
        toast.success("Remember that using Ai features means that your information may be used to train Ai models. Use it responsibly.")
        break;
    }
  }
}

function findSetting(sections: Section[], id: string): Setting | null {
  for (const section of sections) {
    for (const setting of section.sectionSettings) {
      if (setting.id === id) {
        return setting;
      }
    }
    if (section.subsections) {
      const found = findSetting(section.subsections, id);
      if (found) return found;
    }
  }
  return null;
}
function goToSetting(id: string) {
  const el = document.getElementById(id);
  el?.scrollIntoView({ behavior: 'smooth', block: 'start' });
}
function changeSectionVisibility(sections: Section[], id: string, value: boolean) {
  for (let section of sections) {
    if (section.id == id) {
      section.show = value;
      return true;
    }
    if (section?.subsections) {
      if (changeSectionVisibility(section.subsections, id, value)) return true;
    }
  }
  return false;
}

function handleVisibilityChange(id: string, value: boolean) {
  if (!settingList.value) return;

  changeSectionVisibility(settingList.value.sections, id, value);
}

function getElementVisibility(
  sections: Section[] | null | undefined,
  id: string
): boolean | undefined {
  if (!sections) return undefined;
  for (let section of sections) {
    if (section.id === id) {
      return section.show;
    }

    if (section.subsections) {
      const val = getElementVisibility(section.subsections, id);
      if (val !== undefined) return val;
    }
  }
  return undefined;
}

function handleBlur() {
  if (settingList.value == null) return;
  if (searchText.value.length == 0) {
    initSettingVisibility(settingList.value.sections, true);
  }
}
function hide() {
  if (settingList.value == null) return;
  if (searchText.value.length == 0) {
    initSettingVisibility(settingList.value.sections, false);
  }
}
function closeLogs() {
  showLogs.value = false;
}

function askForPassword(): Promise<string | null> {
  showPasswordInput.value = true;
  return new Promise((resolve) => {
    passwordResolver.value = resolve;
  });
}
function handlePassword(p: string) {
  passwordResolver.value?.(p);
  passwordResolver.value = null;
}

function handlePasswordCancel() {
  showPasswordInput.value = false;
  passwordResolver.value?.(null);
  passwordResolver.value = null;
}

async function handleUsername(newUsername: string) {
  usernameLoading.value = true;
  await invoke<void>('change_username', { newUsername: newUsername });
  if (!username) return;
  username.value = newUsername;
  authStore.$patch({
    loggedInUsername: newUsername,
  });
  usernameLoading.value = false;
  showUsernameInput.value = false;
  return;
}
function handleUsernameCancel() {
  showUsernameInput.value = false;
  newUsername.value = '';
}
</script>

<template>
  <div class="relative flex h-full min-h-0 flex-col overflow-hidden px-[10%]">
    <ArrowBigLeftDash
      class="absolute left-[2%] top-[93%] text-note-paprika/80 transition-transform duration-200 hover:scale-95"
      @click="redirect"
    />

    <LogPreview
      v-if="showLogs"
      class="absolute left-[50%] top-[50%] translate-x-[-50%] translate-y-[-50%]"
      :log-content="logContents"
      @close-logs="closeLogs()"
    ></LogPreview>

    <PasswordInput
      :loading="codesLoading"
      v-if="showPasswordInput"
      class="absolute left-[50%] top-[50%] translate-x-[-50%] translate-y-[-50%]"
      @submit-password="handlePassword"
      @cancel-password="handlePasswordCancel"
    ></PasswordInput>

    <ChangeUsername
      :loading="usernameLoading"
      v-if="showUsernameInput"
      class="absolute left-[50%] top-[50%] translate-x-[-50%] translate-y-[-50%]"
      @submit-username="handleUsername"
      @cancel-username="handleUsernameCancel"
    ></ChangeUsername>

    <header class="shrink-0 pb-4 pt-8">
      <div class="flex items-start justify-between gap-8">
        <div class="flex h-[27vh] min-h-60 flex-col justify-between">
          <div class="flex flex-col">
            <h1
              class="text-4xl font-semibold tracking-tight text-note-ivory lg:text-5xl xl:text-6xl"
            >
              Settings of
              <span class="text-note-paprika">{{ username }}</span>
            </h1>
            <p class="mt-3 text-xs tracking-widest text-note-pumice/25">#{{ id }}</p>
          </div>

          <span
            class="flex h-10 w-80 items-center rounded-md border-2 border-note-pumice/50 bg-black/40 p-2 transition duration-1000 ease-out focus-within:border-note-paprika/80 focus-within:bg-black/60"
          >
            <input
              class="w-[90%] select-none bg-note-graphite text-note-ivory outline-none transition duration-1000 ease-out placeholder:text-note-pumice/70 focus:border-transparent focus:bg-black/50 focus:shadow-none focus:outline-none focus:ring-0"
              type="text"
              placeholder="Search..."
              @input="search"
              @blur="handleBlur"
              v-model="searchText"
              @focus="hide"
            />
            <Search class="ml-2 shrink-0 text-note-paprika" />
          </span>
        </div>

        <div
          class="flex h-[27vh] min-h-60 w-[28%] min-w-56 shrink-0 flex-col rounded-xl border border-note-pumice/20 bg-note-graphite/80 px-4 py-4"
        >
          <div class="mb-4 flex items-center gap-2">
            <div
              class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-note-paprika/20 text-note-glow"
            >
              <InfoIcon class="h-4 w-4" />
            </div>
            <div class="flex min-w-0 flex-col">
              <span class="text-sm font-semibold text-note-ivory">Account overview</span>
              <span class="text-xs text-note-pumice/70">
                Quick access to your most important settings
              </span>
            </div>
          </div>

          <div
            class="flex flex-1 flex-col justify-between overflow-hidden rounded-lg border border-note-pumice/10 bg-black/40 px-3 py-2"
          >
            <button
              v-for="setting in cardSettings"
              :key="setting.id"
              type="button"
              class="flex w-full items-center rounded-md px-2 py-1.5 text-xs text-note-pumice/80 transition-colors hover:bg-black/50 hover:text-note-ivory"
            >
              <span class="flex flex-1 justify-start">{{ setting.label }}</span>
              <span class="flex flex-1 justify-center">{{ setting.currentValue }}</span>
              <span
                @click="goToSetting(setting.id)"
                class="flex flex-1 justify-end text-[11px] uppercase tracking-wide text-note-paprika"
              >
                go to setting
              </span>
            </button>
          </div>
        </div>
      </div>
    </header>

    <ScreenDeviderHorizontal class="mt-6 shrink-0" />

    <div class="mb-2 mt-4 flex shrink-0">
      <Funnel
        class="text-note-pumice/90 transition duration-500 ease-out hover:text-note-paprika"
        @click="showFilters"
      />
      <template v-if="showFilter">
        <div
          v-for="filter in filters"
          :key="filter"
          class="ml-4 flex h-fit w-44 border-note-ivory"
        >
          <CheckboxInput
            :checked="getElementVisibility(settingList?.sections, filter) ?? true"
            :id="filter"
            @visibility-changed="
              (id, value) => {
                handleVisibilityChange(id, value);
              }
            "
          ></CheckboxInput>
        </div>
      </template>
    </div>

    <main class="mb-4 mt-4 flex min-h-0 flex-1 flex-col gap-4">
      <div
        class="scrollbar-none min-h-0 flex-1 overflow-y-auto rounded-xl border border-note-pumice/40 bg-black/40 p-4"
      >
        <SectionComp
          v-if="settingList"
          v-for="section in settingList.sections"
          :section="section"
          @setting-changed="handleChange"
        ></SectionComp>
      </div>
    </main>
  </div>
</template>

<style scoped>
.scrollbar-none {
  scrollbar-width: none;
}

.scrollbar-none::-webkit-scrollbar {
  display: none;
}

@media (max-width: 768px) {
  .min-h-60 {
    min-height: 15rem;
  }
}
</style>
