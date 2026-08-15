<script setup lang="ts">
import { Star, Tag } from 'lucide-vue-next'
import ScreenDeviderHorizontal from '../dashboard/ScreenDeviderHorizontal.vue'
</script>

<template>
  <div
    class="absolute bottom-full right-full z-[50] w-60
           rounded-lg border-2 border-note-paprika
           bg-black p-4 shadow-2xl"
  >
    <button
      type="button"
      class="group flex h-9 w-full items-center justify-between
             rounded-md px-3 text-left
             hover:bg-note-graphite/60"
    >
      <span
        class="text-sm font-medium text-note-ivory
               group-hover:text-note-paprika"
      >
        Add to favourites
      </span>

      <Star
        class="h-4 w-4 text-note-glow
               group-hover:scale-110
               group-hover:fill-note-glow"
      />
    </button>

    <ScreenDeviderHorizontal />

    <button
      type="button"
      class="group flex h-9 w-full items-center justify-between
             rounded-md px-3 text-left
             hover:bg-note-graphite/60"
    >
      <span
        class="text-sm font-medium text-note-ivory
               group-hover:text-note-paprika"
      >
        Add tag
      </span>

      <Tag
        class="h-4 w-4 text-note-garnet
               group-hover:scale-110
               group-hover:fill-note-garnet"
      />
    </button>
  </div>
</template>