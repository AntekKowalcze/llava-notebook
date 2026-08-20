<template>
  <div class="h-full min-h-full">
    <Milkdown />
  </div>
</template>

<script setup lang="ts">
import { Milkdown, useEditor } from '@milkdown/vue';
import { Crepe } from '@milkdown/crepe';
import { listener, listenerCtx } from '@milkdown/kit/plugin/listener';
import { editorViewCtx } from '@milkdown/kit/core';
import { useCurrentNoteStore } from '../../stores/currentNoteStore';

const props = defineProps<{ defaultValue: string }>();
const emit = defineEmits<{ (e: 'change', content: string): void }>();

const currentNoteStore = useCurrentNoteStore();

function countWords(text: string): number {
  const cleaned = text.trim();
  if (!cleaned) return 0;
  return cleaned.split(/\s+/).length;
}

useEditor((root) => {
  const crepe = new Crepe({
    root,
    defaultValue: props.defaultValue,
  });

  crepe.editor.use(listener).config((ctx) => {
    const listenerPlugin = ctx.get(listenerCtx);

    listenerPlugin.markdownUpdated((_, markdown) => {
      emit('change', markdown);
    });

    listenerPlugin.updated((ctx) => {
      const view = ctx.get(editorViewCtx);
      if (view) {
        const plainText = view.state.doc.textContent;
        currentNoteStore.words = countWords(plainText);
      }
    });
  });

  return crepe;
});
</script>
