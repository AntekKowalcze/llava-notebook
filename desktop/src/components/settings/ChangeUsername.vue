<script setup lang="ts">
import { ref } from 'vue';
import TextInput from '../auth/forms/TextInput.vue';
import SubmitButton from '../commons/SubmitButton.vue';
import { InputTypes } from '../../types/inputTypes';
import { X } from 'lucide-vue-next';
import LoadingCircle from '../main/LoadingCircle.vue';
defineProps<{ loading: boolean }>();
const username = ref<string>('');
const emit = defineEmits<{
  (e: 'submit-username', username: string): void;
  (e: 'cancel-username'): void;
}>();

function submit() {
  emit('submit-username', username.value);
  username.value = '';
}

function cancel() {
  emit('cancel-username');
  username.value = '';
}
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center">
    <div
      class="absolute inset-0"
      @click="cancel"
    ></div>

    <div
      class="relative z-10 w-[90vw] max-w-md rounded-lg border border-note-pumice/20 bg-black/80 p-6 text-note-ivory shadow-lg"
    >
      <div class="mb-4 flex items-center justify-between">
        <h3 class="text-lg font-semibold">Enter new username</h3>
        <X
          @click="cancel"
          class="h-10 w-10 cursor-pointer p-2 text-note-pumice"
        />
      </div>

      <TextInput
        :placeholder="'Username'"
        :type="InputTypes.Text"
        :name="'text'"
        v-model="username"
      />

      <div class="mt-4 flex items-center justify-end gap-3">
        <button
          @click="cancel"
          class="h-10 rounded-md border border-note-pumice/20 bg-black/30 px-4 text-sm text-note-pumice/80 hover:bg-black/40"
        >
          Cancel
        </button>
        <SubmitButton
          v-if="!loading"
          class="!mt-0 !h-10 !py-0 px-6"
          :content="'Submit'"
          @click="submit"
        />
        <LoadingCircle
          v-if="loading"
          style="transform: scale(0.4); transform-origin: center"
        />
      </div>
    </div>
  </div>
</template>
