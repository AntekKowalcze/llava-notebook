<script setup lang="ts">
import BottomBar from '../components/main/BottomBar.vue';
</script>

<template>
  <div class="flex h-full w-full flex-col">
    <RouterView></RouterView>
    <BottomBar
      :version="'0.1.0'"
      :synced="'synced'"
      class="fixed inset-x-0 bottom-0"
    ></BottomBar>
  </div>
</template>

<style scoped></style>
