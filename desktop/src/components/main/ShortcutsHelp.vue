<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue';
import { X } from 'lucide-vue-next';
import { EDITOR_FORMATTING, SHORTCUTS } from '../../lib/shortcuts';

const emit = defineEmits<{ (e: 'close'): void }>();

const groups = computed(() => {
  const byScope = new Map<string, { keys: string; description: string }[]>();
  for (const shortcut of Object.values(SHORTCUTS)) {
    const list = byScope.get(shortcut.scope) ?? [];
    list.push(shortcut);
    byScope.set(shortcut.scope, list);
  }
  byScope.set('Formatting (editor)', [...EDITOR_FORMATTING]);
  return [...byScope.entries()];
});

function closeOnEscape(event: KeyboardEvent) {
  if (event.key === 'Escape') emit('close');
}

onMounted(() => window.addEventListener('keydown', closeOnEscape));
onUnmounted(() => window.removeEventListener('keydown', closeOnEscape));
</script>

<template>
  <div
    class="fixed inset-0 z-[200] flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm"
    @mousedown.self="emit('close')"
  >
    <div
      class="flex max-h-[85vh] w-full max-w-3xl flex-col overflow-hidden rounded-2xl border-2 border-note-paprika bg-black shadow-2xl"
    >
      <div class="flex items-center justify-between border-b border-note-pumice/15 px-5 py-4">
        <p class="text-lg font-semibold text-note-ivory">Keyboard shortcuts</p>
        <button
          type="button"
          class="rounded-md p-1 text-note-pumice/70 hover:bg-note-graphite hover:text-note-paprika"
          @click="emit('close')"
        >
          <X class="h-5 w-5" />
        </button>
      </div>

      <div
        class="grid grid-cols-1 gap-x-8 gap-y-6 overflow-y-auto px-5 py-5 [scrollbar-width:none] md:grid-cols-2"
      >
        <section
          v-for="[scope, shortcuts] in groups"
          :key="scope"
          class="flex flex-col gap-2"
        >
          <p class="text-[11px] font-medium uppercase tracking-widest text-note-paprika/90">
            {{ scope }}
          </p>
          <div
            v-for="shortcut in shortcuts"
            :key="shortcut.keys"
            class="flex items-center justify-between gap-4 text-sm"
          >
            <span class="text-note-ivory/90">{{ shortcut.description }}</span>
            <span class="flex shrink-0 gap-1">
              <kbd
                v-for="part in shortcut.keys.split('+')"
                :key="part"
                class="rounded-md border border-note-pumice/25 bg-note-graphite px-1.5 py-0.5 font-sans text-[11px] text-note-pumice"
              >
                {{ part }}
              </kbd>
            </span>
          </div>
        </section>
      </div>
    </div>
  </div>
</template>
