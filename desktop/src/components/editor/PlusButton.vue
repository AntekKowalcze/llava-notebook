<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue';
import PlusHoverMenu from './PlusHoverMenu.vue';
import { useLayoutStore } from '../../stores/layoutStore';
const isMenuOpen = ref<boolean>(false);
const layoutStore = useLayoutStore();

// F2: open the menu; the menu itself starts the title editor.
watch(
  () => layoutStore.titleEditRequested,
  (requested) => {
    if (requested) isMenuOpen.value = true;
  }
);
const root = ref<HTMLElement | null>(null);

// Close the menu like a native popup: click elsewhere or press Escape.
function closeOnOutsideClick(event: MouseEvent) {
  if (isMenuOpen.value && root.value && !root.value.contains(event.target as Node)) {
    isMenuOpen.value = false;
  }
}

function closeOnEscape(event: KeyboardEvent) {
  if (event.key === 'Escape') isMenuOpen.value = false;
}

onMounted(() => {
  document.addEventListener('mousedown', closeOnOutsideClick);
  document.addEventListener('keydown', closeOnEscape);
});

onUnmounted(() => {
  document.removeEventListener('mousedown', closeOnOutsideClick);
  document.removeEventListener('keydown', closeOnEscape);
});
const emit = defineEmits<{ (e: 'change_encryption_method', to: boolean): void }>();
</script>

<template>
  <div
    ref="root"
    class="relative inline-block"
  >
    <button
      type="button"
      class="group relative flex h-16 w-16 items-center justify-center rounded-full border-2 border-note-paprika bg-black/40 transition-transform duration-150 active:scale-90"
      @click.stop="isMenuOpen = !isMenuOpen"
    >
      <div
        class="relative h-6 w-6 transition-transform duration-500 ease-out group-hover:rotate-90"
      >
        <div
          class="absolute left-1/2 top-1/2 h-6 w-1 -translate-x-1/2 -translate-y-1/2 bg-note-paprika"
        />

        <div
          class="absolute left-1/2 top-1/2 h-1 w-6 -translate-x-1/2 -translate-y-1/2 bg-note-paprika"
        />
      </div>
    </button>

    <PlusHoverMenu
      v-if="isMenuOpen"
      @change_encryption_method="(to) => emit('change_encryption_method', to)"
    />
  </div>
</template>
