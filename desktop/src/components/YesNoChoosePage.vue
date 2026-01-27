<script setup lang="ts">
import FormButtons from '../components/forms/FormButtons.vue';
import FormCard from '../components/forms/FormCard.vue';
import { useRouter } from 'vue-router';
const router = useRouter()
const props = defineProps<{
    subText: string,
    yesLink: string,
    noLink: string,
}>()


function Yes() {
    router.replace(props.yesLink)
}
function No() {
    router.replace(props.noLink)
}

</script>
<template>
    <FormCard header-text="Choose" :sub-text="props.subText">
        <FormButtons content="YES" @click="Yes()" class="w-48 h-20 mt-4"></FormButtons>
        <FormButtons content="NO" @click="No()" class="w-48 h-20"></FormButtons>
        <slot></slot>

    </FormCard>
</template>
