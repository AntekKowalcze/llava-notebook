<script setup lang="ts">
const props = defineProps<{
  headerText: string,
  subText: string,
}>()
</script>
<template>
  <div class="
    relative group
    flex flex-col items-center w-full max-w-xl p-10 mx-auto mt-48
    bg-note-graphite/40 
    backdrop-blur-xl
    border border-note-ivory/10
    rounded-2xl
    shadow-ember min-h-[625px] h-fit
  ">
    <!-- light -->
    <div
      class="absolute inset-x-0 bottom-0 h-3/4 bg-gradient-to-t from-note-garnet/50 to-transparent blur-[50px] rounded-b-2xl -z-10 min-h-[550px]  ">
    </div>
    <!-- Helper background if text not visible on light -->
    <div class="absolute inset-0 bg-note-graphite/40 rounded-2xl -z-10"></div>
    <!-- Card content -->
    <div class="relative z-10 w-full flex flex-col items-center min-h-[600px] py-16">
      <!-- Header -->
      <div class="flex flex-col items-center w-full mb-8 relative">
        <h1
          class="text-5xl font-bold text-transparent bg-clip-text bg-gradient-to-br from-note-ivory to-note-glow pb-2 select-none">
          {{ props.headerText }}
        </h1>
        <p class="text-note-ivory/50 text-sm mt-1 uppercase tracking-widest select-none text-center">
          {{ props.subText }}
        </p>
      </div>
      <!-- Inputy -->
      <div
        class="w-full h-fit p-5 select-none  border-note-ivory/20 rounded flex flex-col items-center justify-center text-note-ivory/30">
        <slot>

        </slot>
      </div>
    </div>
  </div>



</template>


<!-- ask which is better -->
<!-- 
<template>
 <div class="
    relative group
    flex flex-col items-center w-full max-w-xl p-10 mx-auto mt-48
    
    /* GLASS SURFACE */
    bg-note-graphite/40  /* Adjusted opacity */
    backdrop-blur-xl     /* The actual glass effect */
    
    /* EDGES */
    border border-white/10
    shadow-[inset_0_1px_0_0_rgba(255,255,255,0.1)] /* Subtle top highlight only */
    
    /* OUTER GLOW (EMBER) */
    shadow-ember
    
    rounded-2xl
    h-[625px]
    
    /* FIX: Ensure z-index context is local */
    z-10
  ">
  

     <div class="absolute inset-x-0 bottom-0 h-3/4 bg-gradient-to-t from-note-garnet/50 via-note-garnet/10 to-transparent blur-[60px] rounded-b-2xl -z-20 min-h-[550px]"></div>
    
   
    <div class="absolute inset-0 bg-note-graphite/40 rounded-2xl -z-10"></div>

    <div class="relative z-20 w-full flex flex-col items-center min-h-[600px] py-16">
      
      <div class="flex flex-col items-center w-full mb-8 relative">
        <h1 class="text-5xl font-bold text-transparent bg-clip-text bg-gradient-to-br from-note-ivory to-note-glow pb-2 select-none drop-shadow-md">
          {{ props.headerText}}
        </h1>
        <p class="text-note-ivory/60 text-sm mt-1 uppercase tracking-widest select-none font-medium">
          {{props.subText}}
        </p>
      </div>
      
      <div class="w-full h-fit p-5 select-none flex flex-col items-center justify-center text-note-ivory/30">
        <slot></slot>
      </div>

    </div>
  </div>
</template> -->