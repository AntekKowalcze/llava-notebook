```vue
<script setup lang="ts">
import { X, AlertTriangle, Save, Power } from 'lucide-vue-next';

defineProps<{
  visible: boolean;
  message?: string;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'retry'): void;
  (e: 'force-quit'): void;
}>();
</script>

<template>
  <Transition name="force-quit">
    <div
      v-if="visible"
      class="fixed inset-0 z-[9999] flex items-center justify-center"
    >
      <div
        class="absolute inset-0 bg-note-graphite/80 backdrop-blur-sm"
        @click.stop
      />

      <div
        class="relative z-10 w-[420px] max-w-[calc(100vw-2rem)] overflow-hidden rounded-xl border border-note-pumice/10 bg-note-graphite shadow-2xl shadow-black/50"
      >
        <div class="h-1 w-full bg-note-garnet" />

        <button
          type="button"
          class="absolute right-3 top-3 flex h-8 w-8 items-center justify-center rounded-lg text-note-pumice/50 transition-colors hover:bg-note-pumice/10 hover:text-note-ivory active:scale-95"
          title="Close"
          @click="emit('close')"
        >
          <X :size="18" />
        </button>

        <div class="p-6">
          <!-- Icon -->
          <div
            class="mb-5 flex h-12 w-12 items-center justify-center rounded-full bg-note-garnet/15 text-note-garnet"
          >
            <AlertTriangle :size="25" />
          </div>

          <h2 class="pr-8 text-lg font-semibold tracking-tight text-note-ivory">
            Unable to save note
          </h2>

          <p class="mt-2 text-sm leading-6 text-note-pumice/70">
            Your latest changes could not be saved. Closing the application now may cause you to
            lose them.
          </p>

          <p
            v-if="message"
            class="mt-3 rounded-lg border border-note-paprika/20 bg-note-paprika/5 px-3 py-2 text-xs text-note-pumice/60"
          >
            {{ message }}
          </p>

          <div class="mt-6 flex flex-col gap-2">
            <button
              type="button"
              class="flex w-full items-center justify-center gap-2 rounded-lg bg-note-glow px-4 py-2.5 text-sm font-semibold text-note-graphite transition-all hover:brightness-110 active:scale-[0.98]"
              @click="emit('retry')"
            >
              <Save :size="17" />
              Try to save again
            </button>

            <button
              type="button"
              class="flex w-full items-center justify-center gap-2 rounded-lg border border-note-garnet/40 bg-note-garnet/10 px-4 py-2.5 text-sm font-medium text-note-garnet transition-all hover:border-note-garnet/60 hover:bg-note-garnet/20 active:scale-[0.98]"
              @click="emit('force-quit')"
            >
              <Power :size="17" />
              Force quit
            </button>
          </div>

          <p class="mt-4 text-center text-xs text-note-pumice/40">Unsaved changes may be lost.</p>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.force-quit-enter-active,
.force-quit-leave-active {
  transition: opacity 180ms ease;
}

.force-quit-enter-active > div:last-child,
.force-quit-leave-active > div:last-child {
  transition:
    opacity 180ms ease,
    transform 180ms ease;
}

.force-quit-enter-from,
.force-quit-leave-to {
  opacity: 0;
}

.force-quit-enter-from > div:last-child,
.force-quit-leave-to > div:last-child {
  opacity: 0;
  transform: scale(0.96) translateY(8px);
}
</style>
```
