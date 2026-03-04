<script setup lang="ts">
import YesNoChoosePage from '../components/YesNoChoosePage.vue';
const yesLink = "/loading"
const noLink = "/main/"
</script>
<template>
    <!-- TODO  settings -->

    <YesNoChoosePage :yes-link="yesLink" :no-link="noLink" sub-text="Do you want to register online account">
        <RouterLink to="/loginOnline" class="mt-12 text-note-ivory">If you have online account log in!</RouterLink>
    </YesNoChoosePage>
</template>