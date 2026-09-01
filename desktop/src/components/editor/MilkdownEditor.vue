<template>
  <div class="h-full min-h-full">
    <Milkdown />
  </div>
</template>

<script setup lang="ts">
import { Milkdown, useEditor } from '@milkdown/vue';
import { Crepe } from '@milkdown/crepe';

import { editorViewCtx } from '@milkdown/kit/core';

import { invoke } from '@tauri-apps/api/core';

import { useCurrentNoteStore } from '../../stores/currentNoteStore';
import { useToast } from 'vue-toastification';
import { createGoogleProvider } from './googleAiProvider';
import { computed } from 'vue';
import { useUserConfigStore } from '../../stores/userConfig';
const userConfig = useUserConfigStore();
const aiFeaturesOn = computed(()=> {
  console.log(userConfig.config["online.aiFeatures"])
 return  userConfig.config["online.aiFeatures"] == "on"

})
const props = defineProps<{ defaultValue: string }>();

const emit = defineEmits<{
  (e: 'change', content: string): void;
}>();
const toast = useToast();
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

       [Crepe.Feature.AI]: aiFeaturesOn.value,
    },

    featureConfigs: {
      [Crepe.Feature.AI]: {
      provider: createGoogleProvider(),
      },

      [Crepe.Feature.ImageBlock]: {
        onUpload: async (file: File) => {
          const bytes = new Uint8Array(await file.arrayBuffer());
          if (bytes.length > 20 * 1024 * 1024) {
            toast.warning("Attachments over 20mb can not be synced" , {
              timeout: 10000
            })
          }
          try {
          const attachmentId = await invoke<string>('create_attachment', {
            file: Array.from(bytes),
            fileName: file.name,
            mimeType: file.type,
          });
            return `attachment://${encodeURIComponent(attachmentId)}`;

          }catch(err: any){
            if (err.InvalidMimeType){
                toast.warning("Invalid attachment type");
            }
            throw err
          }
         

        },
      },
    },
  });

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
