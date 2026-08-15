<script setup lang="ts"></script>

<template>
  <div style="transform-origin: center; display: inline-block">
    <div
      class="m-auto h-32 w-32 animate-spin rounded-full
             border-[7px]
             border-t-[18px]
             border-b-[18px]
             border-note-paprika
             shadow-[0_0_60px_rgba(249,115,22,0.8)]
             [animation-duration:1.5s]"
    ></div>
  </div>
</template>

<style scoped>
@keyframes borderColor {
  0%,
  100% {
    border-color: #F97316;
  }

  50% {
    border-color: #d77f0c;
  }
}

div > div {
  animation:
    spin 1.7s linear infinite,
    borderColor 2s ease-in-out infinite;
}
</style>