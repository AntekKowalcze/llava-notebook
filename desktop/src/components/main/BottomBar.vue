<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { useUserConfigStore } from '../../stores/userConfig';
import { Lock, CloudOff, LockOpen, HardDrive, Server, Cloud, RefreshCwOff } from 'lucide-vue-next';
import { useMetaStore } from '../../stores/metaStore,';
const userConfig = useUserConfigStore();
const metaStore = useMetaStore()
onMounted(() => {
  void userConfig.init();
});

const encrypted = computed(() => userConfig.config['local.encryption']);
const local = computed(() => userConfig.config['online.sync'] === 'off');
const connected = computed<boolean | null>(()=> metaStore.isConnectedToServer && metaStore.isConnectedToInternet)
const isLocal = computed(() => userConfig.config['local.mode'] === 'off');

//, CloudUpload, CloudCheck, RefreshCw,
defineProps<{ version: string; synced: string }>();
</script>

<template>
  <div
    class="flex h-7 w-full select-none flex-row items-center justify-between border-t border-white/5 bg-black/40 px-4 text-xs"
  >
    <!-- LEFT: placeholder info -->
    <div class="flex items-center gap-3 text-note-pumice">
      <span>Last edited 3 min ago</span>
      <div class="h-3 w-px bg-white/10" />
      <span>342 words</span>
      <div class="h-3 w-px bg-white/10" />
      <span>Markdown</span>
    </div>

    <div class="flex items-center gap-2">
      <!-- online -->
      <div
        class="flex items-center gap-1.5 text-green-500"
        v-if="connected"  
      >
        <Cloud :size="12" />
        <span>Online</span>
      </div>
      <!-- offline -->
      <div
        class="flex items-center gap-1.5 text-note-garnet"
        v-else
      >
        <CloudOff :size="12" />
        <span>Offline</span>
      </div>

 <div class="h-3 w-px bg-white/10" />

      <!-- sync off-->
       <div class="flex items-center gap-1.5 text-note-garnet" v-if="local">
                <RefreshCwOff :size="12"  /><span>Sync off</span>
            </div> 
            

      <!-- syncing -->
      <!-- <div class="flex items-center gap-1.5 text-note-paprika">
                <RefreshCw :size="12" class="animate-spin [animation-duration:1.5s]" /><span>Syncing...</span>
            </div> -->

      <!-- pending -->
      <!-- <div class="flex items-center gap-1.5 text-note-pumice">
                <CloudUpload :size="12" /><span>Pending</span>
            </div> -->

      <!-- synced -->
      <!-- <div class="flex items-center gap-1.5 text-note-glow">
                <CloudCheck :size="12" /><span>Synced 3 min ago</span>
            </div> -->

      <div class="h-3 w-px bg-white/10" />

      <!-- encrypted -->
      <div
        class="flex items-center gap-1 rounded bg-note-glow/10 px-1.5 py-0.5 text-note-glow"
        v-if="encrypted == 'on'"
      >
        <Lock :size="11" />
        <span>Encrypted</span>
      </div>

      <!-- unencrypted -->
      <div
        class="flex items-center gap-1 rounded bg-note-garnet/10 px-1.5 py-0.5 text-note-garnet"
        v-else
      >
        <LockOpen :size="11" />
        <span>Unencrypted</span>
      </div>

      <div class="h-3 w-px bg-white/10" />

      <!-- local -->
      <div
        class="flex items-center gap-1 rounded bg-white/5 px-1.5 py-0.5 text-note-pumice"
        v-if="!isLocal"
      >
        <HardDrive :size="11" />
        <span>Disk</span>
      </div>

      <!-- cloud -->
      <div
        class="flex items-center gap-1 rounded bg-note-paprika/10 px-1.5 py-0.5 text-note-paprika"
        v-else
      >
        <Server :size="11" />
        <span>Cloud</span>
      </div>

      <div class="h-3 w-px bg-white/10" />

      <span class="text-note-pumice/30">v{{ version }}</span>
    </div>
  </div>
</template>
