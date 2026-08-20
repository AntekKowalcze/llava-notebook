import { defineStore } from 'pinia';
import { ref } from 'vue';
import { Note } from '../types/note';

export const useCurrentNoteStore = defineStore('currentNote', () => {
  const currentNote = ref<Note>();
  const words = ref<number>(0);
  return {
    words,
    currentNote,
  };
});
