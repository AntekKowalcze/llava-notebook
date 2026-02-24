<script setup lang="ts">
import FormCard from '../components/forms/FormCard.vue';
import FormButtons from '../components/forms/FormButtons.vue';
import { useToast } from 'vue-toastification';
import { useRouter } from 'vue-router';
import { onMounted, ref } from 'vue';
import { useAuthStore } from '../stores/auth';

const authStore = useAuthStore();
const router = useRouter();
const toast = useToast();
let keys = ref<string[]>([]);
let areCodesShown = ref<boolean>(false)
onMounted(async () => {
    if (!authStore.recoveryKeys) {
        console.log("NO RECOVERY CODES")
        router.replace({ name: 'register' }) // redirect if no codes
        return
    }
    try {
        keys.value = authStore.recoveryKeys;
        console.log(keys)
        authStore.$patch({ recoveryKeys: null })
        keys.value = formatKeys(keys.value);
    } catch (err) {
        toast.error("error" + err);
    }
})


async function next() {
    await router.replace({ name: "choose" });
}

function formatKeys(keys: string[]) {
    return keys.map((key) => {
        let out = "";
        for (let i = 0; i < key.length; i++) {
            if (i !== 0 && i % 4 === 0) out += "-";
            out += key[i];
        }
        areCodesShown.value = true
        return out;

    });
}
function CopyToClipboard() {
    let keysString = keys.value.join(("\n"));
    navigator.clipboard.writeText(keysString);
    toast.success("Codes copied successfully, remember, never show codes to other people and store them in encrypted places", { timeout: 10000 })
}


</script>


<template>
    <FormCard header-text="Recovery Codes"
        sub-text="These are yours recovery codes, save them if so you can restore your account then">
        <ul class="
    list-disc list-outside
    pl-6 space-y-3
   text-note-pumice marker:text-note-paprika
    min-h-[16rem]
  ">
            <!-- TODO different error toasts on register, add save button functionality -->
            <li v-if="!keys.length" class="text-note-pumice marker:text-note-paprika">
                Generating codes…
            </li>

            <li v-else v-for="key in keys" :key="key" class="font-mono tracking-widest">
                {{ key }}
            </li>
        </ul>

        <div class="flex flex-row w-80 justify-between">
            <FormButtons :content="'Copy'" @click="CopyToClipboard" :disabled="!areCodesShown"> </FormButtons>

            <FormButtons :content="'Next'" @click="next"></FormButtons>
        </div>
    </FormCard>
</template>