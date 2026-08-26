<template>
  <div class="h-full min-h-full">
    <Milkdown />
  </div>
</template>

<script setup lang="ts">
import { Milkdown, useEditor } from '@milkdown/vue';

import { Crepe } from '@milkdown/crepe';

import { editorViewCtx } from '@milkdown/kit/core';

import { createOpenAIProvider } from '@milkdown/crepe/llm-providers/openai';

import { invoke } from '@tauri-apps/api/core';

import { useCurrentNoteStore } from '../../stores/currentNoteStore';

const props = defineProps<{ defaultValue: string }>();

const emit = defineEmits<{
  (e: 'change', content: string): void;
}>();

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

    features: {
      [Crepe.Feature.TopBar]: true,

      // [Crepe.Feature.AI]: true,
    },

    featureConfigs: {
      // [Crepe.Feature.AI]: {
      //   provider: createOpenAIProvider({
      //     baseURL: 'https://api.groq.com/openai/',
      //     apiKey: '',
      //     model: 'openai/gpt-oss-20b', // Fast, free model
      //     dangerouslyAllowBrowser: true, // Only for temporary sandbox testing
      //   }),
      // },

      [Crepe.Feature.ImageBlock]: {
        onUpload: async (file: File) => {
          const bytes = new Uint8Array(await file.arrayBuffer());

          const attachmentId = await invoke<string>('create_attachment', {
            file: Array.from(bytes),
            fileName: file.name,
            mimeType: file.type,
          });

          return `attachment://${encodeURIComponent(attachmentId)}`;
        },
      },
    },
  });

  // TODO think about this ai features, and how to responsibly integrate
  // (mayby give rate limmiting for each ip), + it should be toggled by settings
  // + note when making them on in settings that its giving information to model providers

  crepe.on((listener) => {
    listener.markdownUpdated((_, markdown) => {
      emit('change', markdown);
    });

    listener.updated((ctx) => {
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
