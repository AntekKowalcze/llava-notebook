<script setup lang="ts">
import FormCard from '../components/forms/FormCard.vue';
import FormButtons from '../components/forms/FormButtons.vue';
import { useToast } from 'vue-toastification';
import { useRouter } from 'vue-router';
import { onMounted, ref } from 'vue';
import { useAuthStore } from '../stores/auth';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';

const authStore = useAuthStore();
const router = useRouter();
const toast = useToast();
let keys = ref<string[]>([]);
let areCodesShown = ref<boolean>(false);
onMounted(async () => {
  if (!authStore.recoveryKeys) {
    console.log('NO RECOVERY CODES');
    router.replace({ name: 'register' }); // redirect if no codes
    return;
  }
  try {
    keys.value = authStore.recoveryKeys;
    console.log(keys);
    authStore.$patch({ recoveryKeys: null });
    keys.value = formatKeys(keys.value);
  } catch (err) {
    toast.error('error' + err);
  }
});

async function next() {
  await router.replace({ name: 'choose' });
}

function formatKeys(keys: string[]) {
  return keys.map((key) => {
    let out = '';
    for (let i = 0; i < key.length; i++) {
      if (i !== 0 && i % 4 === 0) out += '-';
      out += key[i];
    }
    areCodesShown.value = true;
    return out;
  });
}
async function CopyToClipboard() {
  let keysString = keys.value.join('\n');

  await writeText(keysString);
  toast.success(
    'Codes copied successfully, remember, never show codes to other people and store them in encrypted places',
    { timeout: 10000 }
  );
}
</script>

<template>
  <FormCard header-text="Recovery Codes"
    sub-text="These are yours recovery codes, save them if so you can restore your account then">
    <ul class="min-h-[16rem] list-outside list-disc space-y-3 pl-6 text-note-pumice marker:text-note-paprika">
      <li v-if="!keys.length" class="text-note-pumice marker:text-note-paprika">
        Generating codes…
      </li>

      <li v-else v-for="key in keys" :key="key" class="font-mono tracking-widest">
        {{ key }}
      </li>
    </ul>

    <div class="flex w-80 flex-row justify-between">
      <FormButtons :content="'Copy'" @click="CopyToClipboard" :disabled="!areCodesShown"></FormButtons>

      <FormButtons :content="'Next'" @click="next"></FormButtons>
    </div>
  </FormCard>
</template>
