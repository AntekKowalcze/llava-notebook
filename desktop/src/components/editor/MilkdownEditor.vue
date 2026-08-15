<template>
  <div class="h-full min-h-0 overflow-y-auto ">
       <Milkdown />
  </div>
</template>

<script setup lang="ts">
import { computed ,ref} from 'vue';
import { Milkdown, useEditor } from '@milkdown/vue'
import { Crepe } from '@milkdown/crepe'

import { Editor, rootCtx } from '@milkdown/kit/core'
import { commonmark } from '@milkdown/kit/preset/commonmark'
import { listener, listenerCtx } from '@milkdown/kit/plugin/listener'
import { defineComponent } from 'vue'
const props = defineProps<{defaultValue: string}>()
const emit = defineEmits<{(e: 'change', content: string):void}>()
const { get } = useEditor((root) => {
  const crepe = new Crepe({
    root,
    defaultValue: props.defaultValue,
  })
  crepe.editor.config((ctx) => {
    ctx.get(listenerCtx).markdownUpdated((ctx, markdown) => {
      emit('change', markdown)
    })
  })

  return crepe
})


 // const { get } = useEditor((root) =>
  //     Editor.make()
  //       .config((ctx) => {
  //         ctx.set(rootCtx, root)
  //         // Add markdown listener for auto-save
  //         ctx.get(listenerCtx).markdownUpdated((ctx, markdown) => {
  //           if(!isDirty){
  //             setSafeSaveTimeout()
  //             isDirty = true
  //           }
  //           if(debounceTimeout){
  //             clearTimeout(debounceTimeout)
  //           }
  //           if(!debounceTimeout){
  //             setDebounceTimeout();
  //           }
           

  //             // If 2 second from last update save, if 60 seconds from last save, save 
           




  //         })
  //       })
  //       .use(commonmark)
  //       .use(listener)
  //   )
</script>
