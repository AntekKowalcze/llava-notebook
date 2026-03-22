<script setup lang="ts">
const props = defineProps<{
  content: string,
  disabled?: boolean
}>();
</script>


<template>
  <div class="
      flex justify-center items-center 
      border-2 w-fit px-10 py-4 h-10 rounded-lg mt-14 
      backdrop-blur-sm tracking-wide font-semibold
      transition-all duration-300 ease-out select-none
    " :class="[
      !props.disabled ? `
        border-note-ivory/20 
        bg-black/30 text-note-ivory
        cursor-pointer
        
        /* Hover Effects (Tylko gdy aktywny) */
        hover:border-note-paprika/60     
        hover:bg-note-garnet/20         
        hover:shadow-[0_0_20px_rgba(249,115,22,0.3)]
        hover:-translate-y-1        

        /* Click Effects */
        active:scale-95  
        active:border-note-paprika    
        active:shadow-none active:translate-y-0
      `
        : `
        border-note-ivory/5 bg-black/10 text-note-ivory/20
        cursor-not-allowed
        grayscale opacity-50
        pointer-events-none /* blocks clicking */
      `

    ]" role="button" tabindex="0" @keydown.enter.prevent="$emit('click')" @keydown.space.prevent="$emit('click')">{{
      props.content }}</div>
</template>