<script setup lang="ts">
// import { ref } from 'vue'
// import type { Note } from './types/note'
// import NoteCard from './components/NoteCard.vue';
// import TextArea from './components/TextArea.vue';

// const title = "this is my title"
// const content = ref<string>(`

// Lorem ipsum dolor sit amet, consectetur adipiscing elit. Quisque sem enim, consectetur in sagittis ut, mollis et ex. Suspendisse semper a libero non sagittis. Phasellus et elit eu ante iaculis congue. Donec quis ligula quis dui placerat mollis in eu massa. Aliquam bibendum euismod tellus, et rhoncus ipsum accumsan sed. Curabitur eleifend, erat id aliquam aliquam, libero lectus imperdiet nunc, maximus consectetur nunc lectus non augue. Nulla rhoncus mi vel tellus dignissim vehicula. Quisque facilisis, velit sit amet elementum volutpat, nunc eros placerat odio, in finibus leo turpis nec lectus. Etiam vestibulum consectetur felis, ac malesuada tellus facilisis eu. Suspendisse in ultricies ipsum. In auctor ligula ac dapibus blandit. Praesent gravida turpis massa, in ullamcorper magna ullamcorper nec. Vestibulum in nisi laoreet, congue est sit amet, mollis diam.

// Vivamus porttitor enim in odio elementum porta. Quisque lobortis ex at lectus sollicitudin, sed mollis tellus facilisis. Cras ullamcorper neque vel enim lobortis varius. Mauris id lorem in erat laoreet auctor. Curabitur semper suscipit urna ut faucibus. Cras in orci id elit tincidunt iaculis eget sodales est. Integer sodales lorem a orci pellentesque, sit amet congue sapien feugiat. Praesent eros felis, ultricies eu nunc eget, finibus viverra sapien. Quisque ullamcorper, ex ut volutpat placerat, lectus dui fringilla magna, eu consectetur nulla est sit amet tellus.

// Nam nisl tortor, maximus et magna sodales, ullamcorper ullamcorper eros. Nam eget dolor gravida, consequat enim a, posuere nisl. Suspendisse quis laoreet dui. Quisque dui leo, facilisis ut tincidunt eu, maximus a nunc. Mauris gravida posuere enim, vitae euismod mi euismod nec. Suspendisse potenti. Vivamus rutrum ullamcorper risus eu maximus. Nunc mollis tempor leo, et efficitur ex. Interdum et malesuada fames ac ante ipsum primis in faucibus. Mauris vehicula nunc eget elit posuere, vel tincidunt magna semper. Duis id consectetur lorem.

// Fusce condimentum pellentesque leo sed laoreet. Nunc dapibus imperdiet iaculis. Integer vulputate at augue nec ornare. Vestibulum massa purus, ornare et viverra in, facilisis pellentesque nunc. In ultricies, mi eu tristique bibendum, velit nunc egestas ex, ut tempor massa augue feugiat lorem. Proin et sagittis lacus. Curabitur in egestas est. Fusce sagittis mauris ornare, tempus neque vitae, malesuada erat. Fusce tempor pulvinar justo vel vulputate.

// Vivamus vel turpis lacus. Aenean pellentesque egestas lectus, eget tempor ante tempor in. Duis mi elit, ultricies ac tortor ac, mollis fermentum nulla. Cras nibh nulla, euismod quis lobortis et, euismod a lacus. Donec elementum nisl sed magna condimentum bibendum. Curabitur ac massa nec lacus porta ultricies vel in purus. Mauris eleifend vehicula urna sed consectetur. Vivamus eget nisi quam. In erat justo, aliquet ut libero mattis, consectetur aliquet purus. Nullam dignissim nunc eget dolor semper vehicula. Integer eu hendrerit nulla. Aliquam ac accumsan est. Donec ante ligula, vehicula eu sollicitudin sit amet, feugiat eget eros.

// Maecenas condimentum viverra justo in pulvinar. Cras eu semper magna, vel porta nulla. Nulla eu est posuere, semper augue commodo, maximus diam. Vivamus condimentum vel ligula sed tempor. Integer vel quam nisl. Aenean tincidunt tristique congue. Nullam quis convallis sapien. Sed congue magna at consectetur convallis. Morbi a turpis ante. Morbi sagittis ut arcu eu iaculis. Pellentesque habitant morbi tristique senectus et netus et malesuada fames ac turpis egestas. Ut at elit sed lacus efficitur sagittis at nec sem. Duis non rutrum eros. Morbi et ante ac lacus molestie accumsan vitae ac magna. Quisque maximus eu elit vitae bibendum.

// Curabitur sit amet lobortis tellus. Aliquam efficitur efficitur ornare. Suspendisse sit amet ante imperdiet, tempor mi eu, luctus augue. Aenean sagittis purus a massa feugiat, ut euismod odio porta. Vestibulum auctor urna a eros suscipit tristique. Pellentesque sodales ex accumsan augue varius dignissim. Cras lorem dolor, fringilla quis diam non, dapibus tincidunt magna. Morbi vitae pretium diam, vitae tristique metus. Maecenas lorem justo, auctor sit amet urna et, tempus pulvinar ante. Sed sed ipsum lacus. Fusce elementum justo id viverra viverra. In hac habitasse platea dictumst. Quisque nulla libero, fermentum ut efficitur ut, facilisis in massa. Sed interdum interdum urna quis ullamcorper. Nunc nec ultricies est, vel egestas ligula. Fusce lobortis ligula eget nulla facilisis pharetra.

// Nam congue ligula non malesuada ornare. Nulla egestas lobortis egestas. Nam sagittis nunc mi, non maximus sapien accumsan ut. Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nam sit amet tempus nulla. Sed non lectus ornare, malesuada mauris in, tincidunt eros. Vestibulum facilisis lectus vel erat semper, vel iaculis ligula rhoncus.

// Aliquam vitae diam laoreet nisl maximus iaculis ut et ipsum. Mauris vestibulum pulvinar lectus, vel rhoncus diam malesuada bibendum. Phasellus lobortis justo sit amet varius pharetra. Donec convallis aliquet velit vitae venenatis. Mauris bibendum enim quis tempor aliquam. Phasellus et odio nulla. Donec sagittis neque at magna porttitor, at dignissim nulla suscipit. Donec vehicula auctor lobortis.

// Donec finibus lorem id vestibulum molestie. Cras quis quam fringilla, elementum leo et, elementum magna. Pellentesque habitant morbi tristique senectus et netus et malesuada fames ac turpis egestas. Aliquam erat volutpat. Praesent venenatis nisl eu porta ornare. Integer commodo, massa ac ullamcorper feugiat, diam dui placerat leo, non sagittis eros nibh quis ligula. Praesent et faucibus felis, ut ultrices nunc. Donec et nibh vitae orci commodo tristique. Sed placerat placerat ullamcorper. Sed sagittis sem vitae urna sollicitudin fermentum. Quisque fermentum dapibus ligula vel tempor. Proin vel auctor mauris. Vivamus vel enim non diam interdum auctor. `)
// const notes = ref<Note[]>([
//   {
//     local_id: "2f4f8c3a-9e12-4a7b-8d21-3f9e1c6b5a10",
//     owner_id: "7b1e2d34-5c67-48fa-9a20-8c4e3b2d1f90",
//     name: "Setup projektu",
//     title: "Inicjalizacja Tauri + Vue",
//     summary: "Notatka o stworzeniu nowego projektu Tauri z Vue i TS.",
//     created_at: 1733210000000,
//     updated_at: 1733210000000,
//     is_deleted: false,
//     deleted_at: undefined,
//   },
//   {
//     local_id: "91a7c4de-3f20-4e8b-92d1-6b5a3c2f1e08",
//     owner_id: "7b1e2d34-5c67-48fa-9a20-8c4e3b2d1f90",
//     name: "Model danych",
//     title: "Projekt typu Note",
//     summary: "Definicja pól Note w Rust i TypeScript.",
//     created_at: 1733220000000,
//     updated_at: 1733223600000,
//     is_deleted: false,
//     deleted_at: undefined,
//   },
//   {
//     local_id: "5cd3b8f1-7a24-41e9-8c32-0ab9d6e4f210",
//     owner_id: "7b1e2d34-5c67-48fa-9a20-8c4e3b2d1f90",
//     name: "UI eksperymentalne",
//     title: "Widok listy notatek",
//     summary: "Próba zbudowania listy notatek w App.vue z Tailwindem.",
//     created_at: 1733230000000,
//     updated_at: 1733233600000,
//     is_deleted: false,
//     deleted_at: undefined,
//   },
//   {
//     local_id: "0a9d8c7b-6e5f-4c3a-9b21-8f7e6d5c4b30",
//     owner_id: "7b1e2d34-5c67-48fa-9a20-8c4e3b2d1f90",
//     name: "Soft delete",
//     title: "Logika usuwania notatek",
//     summary: "Eksperyment z deleted_at i is_deleted po 30 dniach.",
//     created_at: 1733240000000,
//     updated_at: 1733240000000,
//     is_deleted: false,
//     deleted_at: 1733326400000, // np. kilka dni po utworzeniu
//   },
//   {
//     local_id: "3e2d1c0b-9a87-4f65-8e43-2d1c0b9a8f76",
//     owner_id: "7b1e2d34-5c67-48fa-9a20-8c4e3b2d1f90",
//     name: "Stara notatka",
//     title: "Do usunięcia",
//     summary: "Notatka testowa, która minęła już okres 30 dni.",
//     created_at: 1728000000000,
//     updated_at: 1728086400000,
//     is_deleted: true,
//     deleted_at: 1728086400000,
//   },
//    {
//     local_id: "3e2d1c0b-9a87-4f65-8e43-2d1c0b9a8f76",
//     owner_id: "7b1e2d34-5c67-48fa-9a20-8c4e3b2d1f90",
//     name: "Stara notatka",
//     title: "Do usunięcia",
//     summary: "Notatka testowa, która minęła już okres 30 dni.",
//     created_at: 1728000000000,
//     updated_at: 1728086400000,
//     is_deleted: true,
//     deleted_at: 1728086400000,
//   },
//    {
//     local_id: "3e2d1c0b-9a87-4f65-8e43-2d1c0b9a8f76",
//     owner_id: "7b1e2d34-5c67-48fa-9a20-8c4e3b2d1f90",
//     name: "Stara notatka",
//     title: "Do usunięcia",
//     summary: "Notatka testowa, która minęła już okres 30 dni.",
//     created_at: 1728000000000,
//     updated_at: 1728086400000,
//     is_deleted: true,
//     deleted_at: 1728086400000,
//   },
//    {
//     local_id: "3e2d1c0b-9a87-4f65-8e43-2d1c0b9a8f76",
//     owner_id: "7b1e2d34-5c67-48fa-9a20-8c4e3b2d1f90",
//     name: "Stara notatka",
//     title: "Do usunięcia",
//     summary: "Notatka testowa, która minęła już okres 30 dni.",
//     created_at: 1728000000000,
//     updated_at: 1728086400000,
//     is_deleted: true,
//     deleted_at: 1728086400000,
//   },
  
// ]);


</script>
<template>
<div class="min-h-dvh w-full bg-note-graphite bg-cover bg-center flex flex-col">
    <router-view />
  </div>
</template>

<!-- <template>
  <main class="min-h-screen bg-note-graphite text-note-custard p-8 flex justify-start" >
   <div class="flex gap-6">

<section class="w-3/4">
  <h1 class="text-slate-200 font-bold text-3xl pl-3">Lista notatek</h1>

<div class="max-h-[90vh] overflow-y-auto scrollbar-none space-y-4 scroll-smooth mt-2 pr-2 pb-4">
   <NoteCard
        v-for="note in notes"
        :key="note.local_id"
        :note="note"
      />
      </div>
    </section>
  
    
   
    </div>
       <TextArea
  :title="notes[0].title"
  :content="content"
/>
  </main>
</template>

 -->
