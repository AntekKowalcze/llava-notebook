<script setup lang="ts">
import {
    NotebookPen,
    Settings,
    UserRound,
    X,
    FileText,
    Star,
    Cloud,
} from "lucide-vue-next";

import { useLayoutStore } from "../stores/layoutStore";
import { useRouter } from "vue-router";
import ScreenDeviderHorizontal from "./dashboard/ScreenDeviderHorizontal.vue";
import IconComponent from "./main/IconComponent.vue";

const layout = useLayoutStore();
const router = useRouter();

function togglePanel() {
    layout.toggleLeftPanel();
}

function redirect(direction: string) {
    switch (direction) {
        case "settings":
            router.push({ name: "settings" });
            break;

        case "dashboard":
            router.push({ name: "dashboard" });
            break;

        case "editor":
            router.push({ name: "editor" });
            break;
    }
}

const recentNotes = [
    { title: "Rust Async Notes", date: "2 min ago" },
    { title: "MongoDB Schema", date: "Today" },
    { title: "Meeting Notes", date: "Today" },
    { title: "Algorithms", date: "Yesterday" },
    { title: "Ideas", date: "Yesterday" },
    { title: "Math", date: "Mon" },
    { title: "Operating Systems", date: "Sun" },
    { title: "Networking", date: "Sat" },
    { title: "Vue Components", date: "Fri" },
    { title: "Tauri Commands", date: "Last week" },
];
</script>

<template>
    <aside
        class="relative flex h-full w-72 shrink-0 flex-col border-r border-note-pumice/10 bg-note-graphite/80 backdrop-blur-xl text-note-ivory z-10">
        <!-- Close -->
        <button @click="togglePanel"
            class="absolute right-3 top-3 flex h-8 w-8 items-center justify-center rounded-lg text-note-pumice/60 transition-all duration-200 hover:bg-note-paprika/10 hover:text-note-paprika active:scale-90">
            <X class="h-5 w-5" />
        </button>

        <!-- Header -->
        <div class="px-6 pt-10 pb-8">
            <h1 class="text-2xl font-semibold tracking-wide text-note-ivory flex">
                <IconComponent class="mr-2" width="w-10" height="h-10"></IconComponent>
                Llava
            </h1>

            <p class="mt-1 text-sm text-note-pumice/50">
                Your safe, personal knowledge space
            </p>

            <!-- Navigation -->
            <div class="mt-8 flex justify-between">
                <button @click="redirect('settings')"
                    class="group flex h-12 w-12 items-center justify-center rounded-xl border border-note-pumice/20 bg-black/40 transition-all duration-200 ease-out hover:-translate-y-0.5 hover:border-note-paprika/50 hover:bg-note-paprika/10 active:translate-y-[1px] active:scale-95">
                    <Settings
                        class="h-6 w-6 text-note-pumice/80 transition-colors duration-200 group-hover:text-note-paprika" />
                </button>

                <button @click="redirect('dashboard')"
                    class="group flex h-12 w-12 items-center justify-center rounded-xl border border-note-pumice/20 bg-black/40 transition-all duration-200 ease-out hover:-translate-y-0.5 hover:border-note-paprika/50 hover:bg-note-paprika/10 active:translate-y-[1px] active:scale-95">
                    <UserRound
                        class="h-6 w-6 text-note-pumice/80 transition-colors duration-200 group-hover:text-note-paprika" />
                </button>

                <button @click="redirect('editor')"
                    class="group flex h-12 w-12 items-center justify-center rounded-xl border border-note-pumice/20 bg-black/40 transition-all duration-200 ease-out hover:-translate-y-0.5 hover:border-note-paprika/50 hover:bg-note-paprika/10 active:translate-y-[1px] active:scale-95">
                    <NotebookPen
                        class="h-6 w-6 text-note-pumice/80 transition-colors duration-200 group-hover:text-note-paprika" />
                </button>
            </div>
        </div>

        <!-- Divider -->
        <ScreenDeviderHorizontal class="mt-0"></ScreenDeviderHorizontal>
        <!-- Recent Notes -->
        <div class="flex-1 overflow-y-auto px-5 py-6">
            <div class="mb-4 flex items-center gap-2">
                <span class="text-note-paprika text-lg">●</span>

                <h2 class="text-xs font-semibold uppercase tracking-[0.25em] text-note-pumice/55">
                    Recent Notes
                </h2>
            </div>

            <div class="space-y-1">
                <button v-for="note in recentNotes" :key="note.title"
                    class="group flex w-full items-center justify-between rounded-xl border border-transparent px-3 py-3 text-left transition-all duration-200 hover:border-note-paprika/20 hover:bg-white/[0.04]">
                    <div class="flex min-w-0 items-center gap-3">
                        <div
                            class="flex h-8 w-8 items-center justify-center rounded-lg bg-white/[0.03] transition-colors group-hover:bg-note-paprika/10">
                            <FileText class="h-4 w-4 text-note-pumice/50 group-hover:text-note-paprika" />
                        </div>

                        <span class="truncate text-sm text-note-pumice/90 group-hover:text-note-ivory">
                            {{ note.title }}
                        </span>
                    </div>

                    <span class="ml-2 whitespace-nowrap text-xs text-note-pumice/35 group-hover:text-note-pumice/60">
                        {{ note.date }}
                    </span>
                </button>
            </div>
        </div>

        <!-- Bottom Card -->
        <div class="p-5">
            <div class="rounded-2xl border border-note-pumice/10 bg-white/[0.03] p-4">
                <p class="mb-4 text-xs font-semibold uppercase tracking-[0.25em] text-note-pumice/45">
                    Workspace
                </p>

                <div class="space-y-3 text-sm">
                    <div class="flex items-center justify-between">
                        <div class="flex items-center gap-2 text-note-pumice/65">
                            <FileText class="h-4 w-4" />
                            <span>Notes</span>
                        </div>

                        <span class="text-note-ivory">127</span>
                    </div>

                    <div class="flex items-center justify-between">
                        <div class="flex items-center gap-2 text-note-pumice/65">
                            <Star class="h-4 w-4" />
                            <span>Favorites</span>
                        </div>

                        <span class="text-note-ivory">6</span>
                    </div>

                    <div class="flex items-center justify-between">
                        <div class="flex items-center gap-2 text-note-pumice/65">
                            <Cloud class="h-4 w-4" />
                            <span>Status</span>
                        </div>

                        <span class="flex items-center gap-2 text-note-glow">
                            <span class="h-2 w-2 rounded-full bg-note-glow"></span>
                            Synced
                        </span>
                    </div>
                </div>
            </div>
        </div>
    </aside>
</template>