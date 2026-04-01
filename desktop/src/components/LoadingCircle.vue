<script setup lang="ts"></script>

<template>
  <!-- Outer wrapper receives any parent scale transform -->
  <div style="transform-origin: center; display: inline-block;">
    <!-- Inner element performs the spin animation so parent transforms don't override it -->
    <div
      class="m-auto h-32 w-32 animate-spin rounded-full border-[7px] border-b-[18px] border-t-[18px] border-note-paprika border-b-amber-500 shadow-[0_0_60px_rgba(249,115,22,0.8)] [animation-duration:1.5s]">
    </div>
  </div>
</template>
