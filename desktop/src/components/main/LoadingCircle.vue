<script setup lang="ts"></script>

<template>
  <div class="spinner-wrapper">
    <div class="spinner"></div>
  </div>
</template>

<style scoped>
.spinner-wrapper {
  display: inline-block;
  transform-origin: center;
}

.spinner {
  width: 8rem;
  height: 8rem;
  margin: auto;

  border: 7px solid #f97316;
  border-top-width: 18px;
  border-bottom-width: 18px;
  border-radius: 50%;

  box-shadow: 0 0 60px rgba(249, 115, 22, 0.8);

  animation:
    my-spin 2s linear infinite,
    borderColor 2s ease-in-out infinite;
}

@keyframes my-spin {
  from {
    transform: rotate(0deg);
  }

  to {
    transform: rotate(360deg);
  }
}

@keyframes borderColor {
  0%,
  100% {
    border-color: #f97316;
  }

  50% {
    border-color: #d77f0c;
  }
}
</style>