<script setup lang="ts">
import SubmitButton from '../commons/SubmitButton.vue';
import FormCard from './forms/FormCard.vue';
import { useRouter } from 'vue-router';
const router = useRouter();
const props = defineProps<{
  subText: string;
  yesLink: string;
  noLink: string;
}>();

function Yes() {
  router.replace(props.yesLink);
}
function No() {
  router.replace(props.noLink);
}
</script>
<template>
  <FormCard
    header-text="Choose"
    :sub-text="props.subText"
  >
    <SubmitButton
      content="YES"
      @click="Yes()"
      class="mt-4 h-20 w-48"
    ></SubmitButton>
    <SubmitButton
      content="NO"
      @click="No()"
      class="h-20 w-48"
    ></SubmitButton>
    <slot></slot>
  </FormCard>
</template>
