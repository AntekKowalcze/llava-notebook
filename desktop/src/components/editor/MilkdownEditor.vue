<template>
  <div class="h-full min-h-0 overflow-y-auto ">
       <Milkdown />
  </div>
</template>

<script setup lang="ts">
import { computed ,ref} from 'vue';
import { Milkdown, useEditor } from '@milkdown/vue'
import { Crepe } from '@milkdown/crepe'

const date = ref<Date>(new Date());
const h = computed(() => date.value.getHours());

const defaultValue = computed(() => {

  if (h.value < 6) {
    return "# Deep night thoughts? 🌌\n\n> The rest of the world is sleeping, but your mind is awake. \n\nJot down your late-night inspiration before it fades...";
  } 
  else if (h.value >= 6 && h.value < 12) {
    return "# Good morning! ☀️\n\nA fresh day, a blank canvas. **What are we focusing on today?**\n\n- [ ] Task 1\n- [ ] Task 2";
  } 
  else if (h.value >= 12 && h.value < 18) {
    return "# Good afternoon! ☕\n\nMid-day inspiration striking? \n\nDrop your notes, ideas, or meeting summaries right here.";
  } 
  else if (h.value >= 18 && h.value < 22) {
    return "# Good evening! 🌇\n\nWinding down? It's the perfect time to reflect on the day or plan for tomorrow.";
  } 
  else {
return "# Entering Stealth Mode 🥷\n\nDistractions are asleep. It's just you, the keyboard, and the glow of the screen. \n\n**Execute your final thoughts for the day:**";  }
});
useEditor((root) => {
  return new Crepe({
    root,
    defaultValue: defaultValue.value
  })
})
</script>
