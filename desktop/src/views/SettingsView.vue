<script setup lang="ts">
import { Funnel, InfoIcon, Search } from 'lucide-vue-next';
import { ArrowBigLeftDash } from 'lucide-vue-next';
import ScreenDeviderHorizontal from '../components/dashboard/ScreenDeviderHorizontal.vue';
import { useAuthStore } from '../stores/auth';
import { useRouter } from 'vue-router';
import { invoke } from '@tauri-apps/api/core';
import { onMounted } from 'vue';
import { Section, UserConfig } from '../types/settingTypes';
import SectionComp from '../components/settings/SectionComp.vue';
import { ref } from 'vue'
import { useToast } from 'vue-toastification';
const toast = useToast()
const authStore = useAuthStore();
const router = useRouter();
const settingList = ref<UserConfig | null>(null);
const cardSettings: string[] = [
  'important setting',
  'second important',
  'third ',
  'important setting',
  'second important',
]; //pass as prop choosen userConfig setting (like settingsList][0][0] etc and it will change contents automaticly)
const username = authStore.loggedInUsername;
const id = authStore.loggedInUserId;
function search() {
  // TODO: Implement Metaphone algorithm
}
function redirect() {
  router.replace("/main/");
}

onMounted(async () => {
  try {
    settingList.value = await invoke<UserConfig>("get_config_data");
  } catch (e) {

    console.warn("get_config_data failed:", e);
    toast.error("failed to get current config")
  }
})

function showFilters() {
  // TODO here use checkboxes with labels
}
async function handleChange(id: string, value: string) {
  console.log("updating")
  if (!settingList.value) return
  findAndUpdate(settingList.value.sections, id, value)
  console.log(settingList)
  console.log("updated")
  try {
    await invoke('update_settings', { userConfig: settingList.value })

  } catch (err) {
    toast.error("Failed to save config")
  }
}

function findAndUpdate(sections: Section[], id: string, value: string): boolean {
  for (const section of sections) {
    for (const setting of section.sectionSettings) {
      if (setting.id === id) {
        setting.currentValue = value
        return true
      }
    }
    if (section.subsections) {
      if (findAndUpdate(section.subsections, id, value)) return true
    }
  }
  return false
}

</script>

<template>
  <div class="relative h-screen overflow-hidden flex flex-col px-[10%]">

    <ArrowBigLeftDash
      class="text-note-paprika/80 absolute top-[93%] left-[2%] transition-transform duration-200 hover:scale-95"
      @click="redirect" />

    <header class="shrink-0 pt-8 pb-4">
      <div class="flex justify-between items-start gap-8">
        <div class="flex flex-col justify-between h-[27vh] min-h-60">
          <div class="flex flex-col">
            <h1 class="text-note-ivory text-4xl lg:text-5xl xl:text-6xl font-semibold tracking-tight">
              Settings of <span class="text-note-paprika">{{ username }}</span>
            </h1>
            <p class="text-note-pumice/25 text-xs tracking-widest mt-3">
              #{{ id }}
            </p>
          </div>

          <span class="flex items-center bg-black/40 border-note-pumice/50 border-2 w-80 h-10 p-2 rounded-md
                       transition duration-1000 ease-out focus-within:border-note-paprika/80
                       focus-within:bg-black/60">
            <input class="bg-note-graphite text-note-ivory outline-none transition duration-1000 ease-out
                         focus:outline-none focus:ring-0 focus:border-transparent focus:shadow-none
                         focus:bg-black/50 placeholder:text-note-pumice/70 select-none w-[90%]" type="text"
              placeholder="Search..." @input="search" />
            <Search class="ml-2 text-note-paprika shrink-0" />
          </span>
        </div>


        <div class="flex flex-col shrink-0 bg-note-graphite/80 border border-note-pumice/20 rounded-xl
                    px-4 py-4 w-[28%] min-w-56 h-[27vh] min-h-60">
          <div class="flex items-center gap-2 mb-4">
            <div class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full
                        bg-note-paprika/20 text-note-glow">
              <InfoIcon class="h-4 w-4" />
            </div>
            <div class="flex flex-col min-w-0">
              <span class="text-sm font-semibold text-note-ivory">Account overview</span>
              <span class="text-xs text-note-pumice/70">
                Quick access to your most important settings
              </span>
            </div>
          </div>

          <div class="flex-1 rounded-lg bg-black/40 border border-note-pumice/10
                      px-3 py-2 flex flex-col justify-between overflow-hidden">
            <button v-for="setting in cardSettings" :key="setting" type="button" class="flex items-center rounded-md px-2 py-1.5 text-xs text-note-pumice/80
                           hover:text-note-ivory hover:bg-black/50 transition-colors w-full">
              <span class="flex-1 flex justify-start">{{ setting }}</span>
              <span class="flex-1 flex justify-center">value</span>
              <span class="flex-1 flex justify-end text-note-paprika text-[11px] tracking-wide uppercase">
                go to setting
              </span>
            </button>
          </div>
        </div>
      </div>
    </header>

    <ScreenDeviderHorizontal class="shrink-0" />

    <div class="shrink-0 my-2">
      <Funnel class="text-note-pumice/90 transition duration-500 ease-out hover:text-note-paprika"
        @click="showFilters" />
    </div>


    <main class="flex-1 min-h-0 flex flex-col gap-4 pb-6 my-4">
      <div
        class="flex-1 min-h-0 w-full bg-black/40 rounded-xl border border-note-pumice/40 overflow-y-auto scrollbar-none p-4">
        <SectionComp v-if="settingList" v-for="section in settingList.sections" :section="section"
          @setting-changed="handleChange">

        </SectionComp>

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





<!-- my idea for filtration / search, every element on list is v-if, and has its representation in js object like show, option, value,  fuzzy or mayby i should use Soundex or Methaphone, and i will use fuzzy in note content lookup words and parent and its in the representation of section which also has show, availabale options [list of previus described objects], fuzy words, then on filter we have checkboxes which after click is changin show to true or false and v-if is rendering or not, same with serach, serach returns list of "options or names"  that needs to be displayed and we check them to true, rest to false, also tell me should i store fuzzy words on backend or on frontend and pass them or maybe here and here, i will need to create rust struct also for better serialization-->
