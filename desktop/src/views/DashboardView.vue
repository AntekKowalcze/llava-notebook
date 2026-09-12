<script setup lang="ts">
import IconComponent from '../components/main/IconComponent.vue';
import noteSvg from '../assets/notepad.svg';
import { ref } from 'vue';
import { useAuthStore } from '../stores/auth';
import { computed } from 'vue';
import { onUnmounted } from 'vue';
import ScreenDeviderHorizontal from '../components/dashboard/ScreenDeviderHorizontal.vue';
import DashboardNumberCard from '../components/dashboard/DashboardNumberCard.vue';
import { useRouter } from 'vue-router';
import { LockIcon, Calendar, ArrowBigLeftDash } from 'lucide-vue-next';
import { invoke } from '@tauri-apps/api/core';
import ActivityMapSquare from '../components/dashboard/ActivityMapSquare.vue';
import { onMounted } from 'vue';

type ActivityRecord = {
  numberOfEditions: number;
  date: string;
};
const router = useRouter();
type LastEdited = [string, string, string]; // [title, note_id, date]
type FavouriteTag = [string, string]; // [tag_name, color]

type DashboardData = {
  numberOfNotes: number;
  numberOfEncryptedNotes: number;
  accountCreation: number;
  activityVec: ActivityRecord[];
  lastThreeEdited: LastEdited[];
  favouriteTags: FavouriteTag[];
};

const authStore = useAuthStore();
const username = computed(() => authStore.loggedInUsername);
const userId = authStore.loggedInUserId;
const date = ref<Date>(new Date());
const numberOfNotes = ref<number>(0);
const numberOfEncrypted = ref<number>(0);
const accountAgeDays = ref<number>(0);
const lastThreeEdited = ref<LastEdited[]>([]);
const favouriteTags = ref<FavouriteTag[]>([]);

let accountAge: number = 0;
const hours = computed(() => date.value.getHours());
const dateFormatted = computed(() =>
  new Intl.DateTimeFormat('en-GB', {
    weekday: 'long',
    day: 'numeric',
    month: 'short',
  }).format(date.value)
);
const activityMap = ref<Record<string, number>>({});

onMounted(async () => {
  try {
    let dashboardData = await invoke<DashboardData>('get_dashboard_data', {
      userUuid: userId,
    });
    numberOfNotes.value = dashboardData.numberOfNotes;
    numberOfEncrypted.value = dashboardData.numberOfEncryptedNotes;
    accountAge = dashboardData.accountCreation;
    accountAgeDays.value = Math.floor((Date.now() - accountAge) / (1000 * 60 * 60 * 24));
    lastThreeEdited.value = dashboardData.lastThreeEdited;
    favouriteTags.value = dashboardData.favouriteTags;
    const map: Record<string, number> = {};
    for (const rec of dashboardData.activityVec) {
      map[rec.date] = rec.numberOfEditions;
    }
    activityMap.value = map;
  } catch (err) {
    console.error(err);
  }
});
const greeting = computed(() => {
  if (hours.value < 6) return 'Burning the midnight oil,';
  if (hours.value < 12) return 'Good morning,';
  if (hours.value < 18) return 'Good afternoon,';
  if (hours.value < 22) return 'Good evening,';
  return 'Up late,';
});
const interval = setInterval(() => {
  date.value = new Date();
  accountAgeDays.value = Math.floor((Date.now() - accountAge) / (1000 * 60 * 60 * 24));
}, 60_000);
type DayCell = {
  date: string;
  contributions: number;
};
const weeks = computed<DayCell[][]>(() => {
  const result: DayCell[][] = [];
  const today = new Date();
  const start = new Date();
  start.setDate(today.getDate() - 364); // 365 dni wstecz
  let current = new Date(start);
  let week: DayCell[] = [];

  while (current <= today) {
    const y = current.getFullYear();
    const m = String(current.getMonth() + 1).padStart(2, '0');
    const d = String(current.getDate()).padStart(2, '0');
    const key = `${y}-${m}-${d}`;

    week.push({
      date: key,
      contributions: activityMap.value[key] ?? 0,
    });

    if (week.length === 7) {
      result.push(week);
      week = [];
    }
    current.setDate(current.getDate() + 1);
  }
  if (week.length > 0) {
    result.push(week);
  }
  return result;
});

const weekdayLabels: string[] = ['Sun', '', 'Tue', '', 'Thu', '', 'Sat'];
const monthLabels: string[] = [
  'January',
  'February',
  'March',
  'April',
  'May',
  'June',
  'July',
  'August',
  'September',
  'October',
  'November',
  'December',
];
let getMonth: number = date.value.getMonth();
let rotatedElements: string[] = monthLabels.splice(0, getMonth);
monthLabels.push(...rotatedElements);
function redirect() {
  router.replace('/main/');
}
function goToNote(noteId: string) {
  router.replace(`/main/editor/${noteId}`);
}
onUnmounted(() => clearInterval(interval));
</script>
<template>
  <div
    class="flex h-full w-full flex-col overflow-y-auto bg-note-graphite px-4 py-5 [-ms-overflow-style:none] [scrollbar-width:none] sm:px-6 sm:py-6 md:px-8 [&::-webkit-scrollbar]:hidden"
  >
    <button
      @click="redirect"
      class="fixed left-[-2%] top-[93%] z-30 flex h-11 w-11 items-center justify-center rounded-full bg-note-graphite/80 text-note-paprika/90 shadow-lg backdrop-blur-md transition-transform duration-200 hover:scale-105 active:scale-90 sm:bottom-6 sm:left-6"
    >
      <ArrowBigLeftDash class="h-6 w-6" />
    </button>

    <div
      class="relative mt-2 flex flex-col items-center justify-center gap-4 py-4 min-[1600px]:flex-row min-[1600px]:gap-0"
    >
      <div class="shrink-0 min-[1600px]:absolute min-[1600px]:left-[6%]">
        <IconComponent
          width="w-24 sm:w-28 lg:w-36"
          height="h-24 sm:h-28 lg:h-36"
          class="object-contain"
        />
      </div>
      <div
        class="min-[1600px]: flex flex-col items-center gap-2 text-center min-[1600px]:text-left"
      >
        <h1
          class="text-2xl font-semibold tracking-tight text-note-ivory sm:text-3xl md:text-4xl min-[1600px]:text-5xl"
        >
          {{ greeting }}
          <span class="text-note-paprika">{{ username }}</span>
        </h1>
        <p class="text-xs tracking-widest text-note-pumice/25">#{{ userId }}</p>
        <p class="text-xs uppercase tracking-widest text-note-ivory/80 sm:text-sm">
          {{ dateFormatted }}
        </p>
      </div>
    </div>

    <ScreenDeviderHorizontal class="my-4" />

    <div class="mx-auto flex w-full max-w-6xl flex-col">
      <div class="mb-4">
        <p class="text-center text-2xl font-semibold text-note-ivory sm:text-left sm:text-3xl">
          This is your activity in Llava app
        </p>
      </div>

      <div
        class="mt-2 grid w-full grid-cols-1 justify-items-center gap-4 sm:grid-cols-2 lg:grid-cols-3"
      >
        <DashboardNumberCard
          :text="'number of notes'"
          :count="numberOfNotes"
        >
          <img
            :src="noteSvg"
            class="h-12 w-12 object-contain sm:h-16 sm:w-16"
          />
        </DashboardNumberCard>

        <DashboardNumberCard
          :text="'number of encrypted notes'"
          :count="numberOfEncrypted"
        >
          <LockIcon class="h-12 w-12 text-note-ivory sm:h-16 sm:w-16" />
        </DashboardNumberCard>

        <DashboardNumberCard
          :text="'account age in days'"
          :count="accountAgeDays"
        >
          <Calendar class="h-12 w-12 text-note-ivory sm:h-16 sm:w-16" />
        </DashboardNumberCard>
      </div>

      <div
        class="mt-8 flex flex-col gap-4 rounded-2xl border border-note-pumice/15 bg-note-graphite/60 p-4 backdrop-blur-xl sm:p-5 md:p-6"
      >
        <div class="flex items-baseline justify-between">
          <p class="text-base font-semibold text-note-ivory sm:text-lg md:text-xl">
            Activity heatmap
          </p>
          <p class="text-[10px] uppercase tracking-widest text-note-pumice/60 sm:text-xs">
            Last 365 days
          </p>
        </div>

        <div
          class="overflow-x-auto pb-3 [-ms-overflow-style:none] [-webkit-overflow-scrolling:touch] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
        >
          <div class="inline-flex min-w-max flex-col gap-2">
            <div class="flex gap-3">
              <div class="w-8 shrink-0"></div>
              <div
                class="grid"
                :style="{
                  gridTemplateColumns: `repeat(${weeks.length}, 1rem)`,
                  columnGap: '0.28rem',
                }"
              >
                <span
                  v-for="(label, idx) in monthLabels"
                  :key="idx"
                  class="whitespace-nowrap text-[10px] leading-4 text-note-pumice/60 sm:text-[11px]"
                  :style="{
                    gridColumnStart: Math.min(
                      Math.round((idx * weeks.length) / monthLabels.length) + 1,
                      weeks.length
                    ),
                  }"
                >
                  {{ label.slice(0, 3) }}
                </span>
              </div>
            </div>

            <div class="flex gap-3">
              <div class="flex w-8 shrink-0 flex-col justify-between py-[2px]">
                <span
                  v-for="(label, idx) in weekdayLabels"
                  :key="idx"
                  class="h-4 select-none text-[10px] leading-4 text-note-pumice/60"
                >
                  {{ label }}
                </span>
              </div>

              <div class="inline-grid grid-flow-col grid-rows-7 gap-[0.28rem]">
                <ActivityMapSquare
                  v-for="day in weeks.flat()"
                  :key="day.date"
                  :date="day.date"
                  :numberOfContributions="day.contributions"
                />
              </div>
            </div>
          </div>
        </div>

        <div class="flex items-center gap-2 pt-1 text-[11px] text-note-pumice/70">
          <span>Less</span>
          <div class="flex gap-1">
            <div class="h-3 w-3 rounded-[3px] border border-note-pumice/40 bg-note-graphite" />
            <div class="h-3 w-3 rounded-[3px] bg-note-paprika/40" />
            <div class="h-3 w-3 rounded-[3px] bg-note-paprika/70" />
            <div class="h-3 w-3 rounded-[3px] bg-note-paprika" />
          </div>
          <span>More</span>
        </div>
      </div>

      <ScreenDeviderHorizontal class="my-6" />

      <div class="grid grid-cols-1 gap-4 pb-6 sm:gap-6 md:grid-cols-2">
        <div
          class="flex flex-col gap-3 rounded-2xl border border-note-pumice/20 bg-note-graphite/60 px-4 py-3 sm:px-5 sm:py-4"
        >
          <div class="flex items-center justify-between">
            <p class="text-base font-semibold text-note-ivory sm:text-lg">Recently edited</p>
            <span class="text-[10px] uppercase tracking-widest text-note-pumice/60 sm:text-[11px]">
              last 3 notes
            </span>
          </div>

          <div
            v-if="lastThreeEdited.length"
            class="flex flex-col gap-2"
          >
            <div
              v-for="[title, noteId, ts] in lastThreeEdited"
              :key="noteId + ts"
              class="flex cursor-pointer items-center justify-between rounded-xl border border-note-pumice/15 bg-note-graphite px-3 py-2 transition-colors hover:bg-white/[0.04]"
              @click="goToNote(noteId)"
            >
              <div class="flex min-w-0 flex-col pr-2">
                <span class="truncate text-sm font-medium text-note-ivory">{{ title }}</span>
                <span class="truncate text-xs text-note-pumice/70">
                  {{ noteId }}
                </span>
              </div>
              <span class="whitespace-nowrap text-xs text-note-pumice/60">
                {{ new Date(ts).toLocaleDateString() }}
              </span>
            </div>
          </div>

          <p
            v-else
            class="py-2 text-sm text-note-pumice/50"
          >
            No recent edits yet. Start writing your first note.
          </p>
        </div>

        <div
          class="flex flex-col gap-3 rounded-2xl border border-note-pumice/20 bg-note-graphite/60 px-4 py-3 sm:px-5 sm:py-4"
        >
          <div class="flex items-center justify-between">
            <p class="text-base font-semibold text-note-ivory sm:text-lg">Favourite tags</p>
            <span class="text-[10px] uppercase tracking-widest text-note-pumice/60 sm:text-[11px]">
              top 3
            </span>
          </div>

          <div
            v-if="favouriteTags.length"
            class="flex flex-col gap-2"
          >
            <div
              v-for="([tagName, color], idx) in favouriteTags"
              :key="tagName"
              class="flex items-center justify-between rounded-xl border border-note-pumice/15 bg-note-graphite px-3 py-2"
            >
              <div class="flex min-w-0 items-center gap-2">
                <span
                  class="inline-block h-3 w-3 shrink-0 rounded-full"
                  :style="{ backgroundColor: color }"
                />
                <span class="truncate text-sm font-medium text-note-ivory">
                  {{ tagName }}
                </span>
              </div>
              <span class="text-xs text-note-pumice/60">#{{ idx + 1 }}</span>
            </div>
          </div>

          <p
            v-else
            class="py-2 text-sm text-note-pumice/50"
          >
            No tags yet. Add tags to your notes to see them here.
          </p>
        </div>
      </div>
    </div>
  </div>
</template>
