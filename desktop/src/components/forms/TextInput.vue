<script setup lang="ts">
import { ref, watchEffect } from 'vue';
import { computed } from 'vue';
import iconHidden from '../../assets/inputs/password_hidden.png';
import iconShown from '../../assets/inputs/password_shown.png';
import { InputTypes } from '../../types/inputTypes';
import { type Input } from '../../types/inputTypes';
let props = defineProps<Input>();
const isValid = defineModel<boolean>('isValid');
const inputValue = defineModel<string>({ default: '' });
const currentType = ref<InputTypes>(props.type);
const isPasswordInput = props.type === InputTypes.Password;
const toggleIcon = computed(() => {
  return currentType.value === InputTypes.Password ? iconHidden : iconShown;
});
function toggleVisibility() {
  currentType.value =
    currentType.value === InputTypes.Password ? InputTypes.Text : InputTypes.Password;
}

const requirements = computed(() => [
  { text: 'Minimum 8 characters', met: inputValue.value.length >= 8 },
  { text: 'At least one lowercase letter', met: /[a-z]/.test(inputValue.value) },
  {
    text: 'At least one symbol',
    met: /[!@#$%^&*()_+\-=\[\]{};':"\\|,.<>\/?]/.test(inputValue.value),
  },
  { text: 'At least one uppercase letter', met: /[A-Z]/.test(inputValue.value) },
]);

const showValidation = computed(
  () => isPasswordInput && inputValue.value.length > 0 && props.showValidation
);

watchEffect(() => {
  const allMet = requirements.value.every((req) => req.met);
  isValid.value = allMet;
});
</script>
<template>
  <div class="group relative mt-6 flex w-[60%] flex-col">
    <input
      v-model="inputValue"
      class="h-10 w-full rounded-xl border border-note-ivory/10 bg-black/20 px-4 pr-10 text-note-ivory placeholder-note-ivory/30 outline-none transition-all focus:border-note-paprika/50 focus:bg-black/40"
      :placeholder="props.placeholder"
      :type="currentType"
      :name="props.name"
    />
    <Transition
      name="rotate-fade"
      mode="out-in"
      duration-100
    >
      <img
        v-if="isPasswordInput"
        :src="toggleIcon"
        @click="toggleVisibility"
        :key="currentType"
        class="absolute right-3 top-1/2 h-5 w-4 -translate-y-1/2 cursor-pointer select-none opacity-50 transition-opacity hover:opacity-100"
        alt="Toggle password visibility"
      />
    </Transition>

    <div
      v-if="showValidation"
      class="mt-3 flex origin-top flex-col space-y-1.5 px-2 transition-all duration-300"
    >
      <div
        v-for="(req, index) in requirements"
        :key="index"
        class="flex items-center space-x-2 text-xs transition-colors duration-300"
        :class="req.met ? 'text-note-glow' : 'text-note-ivory/40'"
      >
        <div class="relative flex h-3 w-3 items-center justify-center">
          <div
            v-if="!req.met"
            class="h-1 w-1 rounded-full bg-note-ivory/30"
          ></div>
          <svg
            v-else
            class="h-3.5 w-3.5 text-note-paprika drop-shadow-[0_0_5px_rgba(249,115,22,0.8)]"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="3"
              d="M5 13l4 4L19 7"
            />
          </svg>
        </div>
        <!-- Tekst -->
        <span :class="{ 'font-medium': req.met }">
          {{ req.text }}
        </span>
        <slot></slot>
      </div>
    </div>
  </div>
</template>

<style scoped>
.rotate-fade-enter-active,
.rotate-fade-leave-active {
  transition: all 0.08s ease-out;
}
</style>
