<script setup lang="ts">
import { computed } from 'vue';
import { useUserConfigStore } from '../stores/userConfig';
const userConfig = useUserConfigStore();
const encrypted = computed(() => {
    return userConfig.config["local.encryption"]
})
const local = computed(() => {
    console.log(userConfig.config["local.mode"])
    if (userConfig.config["local.mode"] == "on") {
        return true
    } else {
        return false
    }
})
import {
    Lock,
    CloudOff,
    LockOpen,
    HardDrive,
    Server
} from 'lucide-vue-next'
//, CloudUpload, CloudCheck, RefreshCw, 
defineProps<{ version: string, synced: string }>()
</script>

<template>
    <div
        class="flex flex-row items-center justify-between px-4 w-full h-7 text-xs select-none bg-black/40 border-t border-white/5">

        <!-- LEWA: info o notatce -->
        <div class="flex items-center gap-3 text-note-pumice">
            <span>Last edited 3 min ago</span>
            <div class="w-px h-3 bg-white/10" />
            <span>342 words</span>
            <div class="w-px h-3 bg-white/10" />
            <span>Markdown</span>
        </div>

        <!-- PRAWA: app status -->
        <div class="flex items-center gap-2">

            <!-- offline -->
            <div class="flex items-center gap-1.5 text-note-garnet">
                <CloudOff :size="12" /><span>Offline</span>
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

            <div class="w-px h-3 bg-white/10" />

            <!-- encrypted -->
            <div class="flex items-center gap-1 px-1.5 py-0.5 rounded bg-note-glow/10 text-note-glow"
                v-if="encrypted == 'on'">
                <Lock :size="11" /><span>Encrypted</span>
            </div>

            <!-- unencrypted -->
            <div class="flex items-center gap-1 px-1.5 py-0.5 rounded bg-note-garnet/10 text-note-garnet" v-else>
                <LockOpen :size="11" /><span>Unencrypted</span>
            </div>

            <div class="w-px h-3 bg-white/10" />

            <!-- local -->
            <div class="flex items-center gap-1 px-1.5 py-0.5 rounded bg-white/5 text-note-pumice" v-if="local">
                <HardDrive :size="11" /><span>Local</span>
            </div>

            <!-- cloud -->
            <div class="flex items-center gap-1 px-1.5 py-0.5 rounded bg-note-paprika/10 text-note-paprika" v-else>
                <Server :size="11" /><span>Cloud</span>
            </div>

            <div class="w-px h-3 bg-white/10" />

            <span class="text-note-pumice/30">v{{ version }}</span>
        </div>

    </div>
</template>
