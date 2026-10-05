<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { Cloud, HardDrive } from 'lucide-vue-next';

type StorageUsage = {
  local: {
    notesCount: number;
    notesBytes: number;
    attachmentsCount: number;
    attachmentsBytes: number;
  };
  cloud: { usedBytes: number; reservedBytes: number; limitBytes: number } | null;
  cloudStatus: 'available' | 'notConnected' | 'unavailable';
};

const usage = ref<StorageUsage | null>(null);

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB'];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${value.toFixed(value < 10 ? 1 : 0)} ${units[unit]}`;
}

const cloudPercent = computed(() => {
  const cloud = usage.value?.cloud;
  if (!cloud || cloud.limitBytes <= 0) return 0;
  return Math.min(100, ((cloud.usedBytes + cloud.reservedBytes) / cloud.limitBytes) * 100);
});

const barColor = computed(() => {
  if (cloudPercent.value >= 90) return 'bg-note-garnet';
  if (cloudPercent.value >= 75) return 'bg-note-glow';
  return 'bg-note-paprika';
});

const localTotal = computed(() =>
  usage.value ? usage.value.local.notesBytes + usage.value.local.attachmentsBytes : 0
);

async function loadUsage() {
  try {
    usage.value = await invoke<StorageUsage>('get_storage_usage');
  } catch (err) {
    console.error('Failed to load storage usage:', err);
  }
}

let unlistenSyncFinished: UnlistenFn | null = null;

onMounted(async () => {
  await loadUsage();
  // Uploads and downloads change both numbers.
  unlistenSyncFinished = await listen('sync_finished', () => {
    void loadUsage();
  });
});

onUnmounted(() => {
  unlistenSyncFinished?.();
  unlistenSyncFinished = null;
});
</script>

<template>
  <div
    class="mt-8 flex flex-col gap-5 rounded-2xl border border-note-pumice/15 bg-note-graphite/60 p-4 backdrop-blur-xl sm:p-5 md:p-6"
  >
    <p class="text-base font-semibold text-note-ivory sm:text-lg md:text-xl">Storage</p>

    <div class="flex flex-col gap-2">
      <div class="flex items-center justify-between gap-3">
        <div class="flex items-center gap-2 text-note-ivory">
          <Cloud class="h-5 w-5 text-note-paprika" />
          <span class="text-sm font-medium sm:text-base">Cloud attachments</span>
        </div>
        <span
          v-if="usage?.cloud"
          class="text-xs text-note-pumice/80 sm:text-sm"
        >
          {{ formatBytes(usage.cloud.usedBytes + usage.cloud.reservedBytes) }} of
          {{ formatBytes(usage.cloud.limitBytes) }}
        </span>
      </div>

      <template v-if="usage?.cloud">
        <div class="h-2.5 w-full overflow-hidden rounded-full bg-note-pumice/15">
          <div
            class="h-full rounded-full transition-all duration-500"
            :class="barColor"
            :style="{ width: `${cloudPercent}%` }"
          />
        </div>
        <p class="text-[11px] text-note-pumice/60 sm:text-xs">
          {{ cloudPercent.toFixed(0) }}% used. Only attachments count towards the limit; note text
          is free.
        </p>
      </template>
      <p
        v-else-if="usage?.cloudStatus === 'notConnected'"
        class="text-xs text-note-pumice/60 sm:text-sm"
      >
        Connect an online account with sync turned on to see your cloud storage.
      </p>
      <p
        v-else-if="usage"
        class="text-xs text-note-pumice/60 sm:text-sm"
      >
        Cloud usage is unavailable right now (offline or the server cannot be reached).
      </p>
    </div>

    <div
      v-if="usage"
      class="flex flex-col gap-2 border-t border-note-pumice/10 pt-4"
    >
      <div class="flex items-center justify-between gap-3">
        <div class="flex items-center gap-2 text-note-ivory">
          <HardDrive class="h-5 w-5 text-note-paprika" />
          <span class="text-sm font-medium sm:text-base">On this device</span>
        </div>
        <span class="text-xs text-note-pumice/80 sm:text-sm">{{ formatBytes(localTotal) }}</span>
      </div>
      <div class="flex flex-wrap gap-x-6 gap-y-1 text-[11px] text-note-pumice/60 sm:text-xs">
        <span>
          {{ usage.local.notesCount }} {{ usage.local.notesCount === 1 ? 'note' : 'notes' }} ·
          {{ formatBytes(usage.local.notesBytes) }}
        </span>
        <span>
          {{ usage.local.attachmentsCount }}
          {{ usage.local.attachmentsCount === 1 ? 'attachment' : 'attachments' }} ·
          {{ formatBytes(usage.local.attachmentsBytes) }}
        </span>
      </div>
    </div>
  </div>
</template>
