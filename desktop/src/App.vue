<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const name = ref('')
const greetMsg = ref('')

async function greet() {
  // Wywołanie Tauri command
  greetMsg.value = await invoke('greet', { name: name.value })
}
</script>

<template>
  <div class="flex items-center justify-center min-h-screen bg-gradient-to-br from-indigo-500 to-purple-600">
    <div class="bg-white p-10 rounded-2xl shadow-2xl max-w-md w-full">
      <h1 class="text-4xl font-bold text-gray-900 mb-6 text-center">
        🔥 Llava
      </h1>
      
      <div class="space-y-4">
        <input
          v-model="name"
          type="text"
          placeholder="Enter your name..."
          class="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-indigo-500 focus:border-transparent outline-none"
          @keyup.enter="greet"
        />
        
        <button
          @click="greet"
          class="w-full px-6 py-3 bg-indigo-600 text-white font-semibold rounded-lg hover:bg-indigo-700 transition-colors duration-200"
        >
          Greet
        </button>
        
        <div 
          v-if="greetMsg" 
          class="p-4 bg-green-50 border border-green-200 rounded-lg"
        >
          <p class="text-green-800 text-center font-medium">
            {{ greetMsg }}
          </p>
        </div>
      </div>
    </div>
  </div>
</template>
