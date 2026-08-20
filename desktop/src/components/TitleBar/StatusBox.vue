<script setup lang="ts">
defineProps<{
  currentUser: string;
  onlineAccount: string | null;
  internetConnection: boolean | null;
  serverConnection: boolean | null;
}>();
</script>

<template>
  <div
    class="w-64 rounded-xl border border-note-pumice/10 bg-note-graphite/95 p-4 shadow-2xl backdrop-blur-xl"
  >
    <div class="flex flex-col gap-2.5">
      <div class="flex items-center">
        <span class="w-28 shrink-0 text-sm text-note-pumice">Current user</span>

        <span class="min-w-0 flex-1 truncate text-right text-sm text-note-ivory">
          {{ currentUser }}
        </span>
      </div>

      <!-- Online account -->
      <div class="flex items-center">
        <span class="w-28 shrink-0 text-sm text-note-pumice">email</span>

        <span class="min-w-0 flex-1 truncate text-right text-sm text-note-ivory">
          {{ onlineAccount ? onlineAccount : 'Not connected' }}
        </span>
      </div>

      <!-- Internet -->
      <div class="flex items-center">
        <span class="w-28 shrink-0 text-sm text-note-pumice">Internet</span>

        <span
          class="ml-2 flex flex-1 items-center justify-end gap-1.5 text-xs"
          :class="internetConnection ? 'text-note-glow' : 'text-note-garnet'"
        >
          <span
            class="h-1.5 w-1.5 shrink-0 rounded-full"
            :class="internetConnection ? 'bg-note-glow' : 'bg-note-garnet'"
          />
          {{ internetConnection ? 'Connected' : 'Offline' }}
        </span>
      </div>

      <!-- Server -->
      <div class="flex items-center">
        <span class="w-28 shrink-0 text-sm text-note-pumice">Server</span>

        <span
          class="ml-2 flex flex-1 items-center justify-end gap-1.5 text-xs"
          :class="serverConnection ? 'text-note-glow' : 'text-note-garnet'"
        >
          <span
            class="h-1.5 w-1.5 shrink-0 rounded-full"
            :class="serverConnection ? 'bg-note-glow' : 'bg-note-garnet'"
          />
          {{ serverConnection ? 'Connected' : 'Unavailable' }}
        </span>
      </div>
    </div>
  </div>
</template>
