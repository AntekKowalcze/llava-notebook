<template>
    <div>
        <div class="flex flex-col items-start select-none ">
            <!-- label over the card -->
            <p class="mb-2 text-sm font-medium uppercase tracking-widest text-note-pumice/70 ">
                {{ text }}
            </p>

            <div class="h-32 w-64 rounded-xl border-2 border-note-pumice/40 bg-note-graphite/60
               flex items-center justify-between px-5 ">
                <div class="flex h w-16 items-center justify-center rounded-lg
               ">
                    <slot></slot>
                </div>

                <span class="text-5xl font-semibold text-note-paprika">
                    {{ count }}
                </span>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">


const props = defineProps<{
    count: number,
    text: string
}>()
</script>

<style scoped></style>