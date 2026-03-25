<script setup lang="ts">
import FormButtons from '../components/forms/FormButtons.vue';
import FormCard from '../components/forms/FormCard.vue';
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
    <FormButtons
      content="YES"
      @click="Yes()"
      class="mt-4 h-20 w-48"
    ></FormButtons>
    <FormButtons
      content="NO"
      @click="No()"
      class="h-20 w-48"
    ></FormButtons>
    <slot></slot>
  </FormCard>
</template>
