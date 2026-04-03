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
type LastEdited = [string, string]; // [note_id, date]
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
console.log(authStore.loggedInUserId);
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

onUnmounted(() => clearInterval(interval));
</script>

<template>
  <div class="flex flex-1 flex-col">
    <ArrowBigLeftDash
      class="click:scale-90 absolute left-[2%] top-[93%] text-note-paprika/80 transition-transform duration-200 hover:scale-95"
      @click="redirect"
    ></ArrowBigLeftDash>
    <!-- greeting -->
    <div class="relative mt-4 flex items-center justify-center py-10">
      <div class="absolute left-[12.5%]">
        <IconComponent
          :width="'w-44'"
          :height="'h-44'"
        />
      </div>
      <div class="flex flex-col items-center gap-3">
        <h1 class="text-5xl font-semibold tracking-tight text-note-ivory">
          {{ greeting }}
          <span class="text-note-paprika">{{ username }}</span>
        </h1>
        <p class="mb-4 text-xs tracking-widest text-note-pumice/25">#{{ userId }}</p>
        <p class="text-sm uppercase tracking-widest text-note-ivory/80">
          {{ dateFormatted }}
        </p>
      </div>
    </div>

    <ScreenDeviderHorizontal />
    <!-- reszta dashboardu niżej -->
    <div class="flex-1 flex-col items-start">
      <div class="mb-4 h-fit">
        <p class="mb-0 ml-[12.5%] mt-10 text-3xl font-semibold text-note-ivory">
          This is your activity in Llava app
        </p>
      </div>

      <div class="mt-0 flex h-1/5 w-full flex-row items-center justify-evenly">
        <!-- number of notes card -->
        <DashboardNumberCard
          :text="'number of notes'"
          :count="numberOfNotes"
        >
          <img
            :src="noteSvg"
            class="h-16 w-16"
          />
        </DashboardNumberCard>

        <DashboardNumberCard
          :text="'number of encrypted notes'"
          :count="numberOfEncrypted"
        >
          <LockIcon class="h-16 w-16 text-note-ivory"></LockIcon>
        </DashboardNumberCard>
        <DashboardNumberCard
          :text="'account age in days'"
          :count="accountAgeDays"
        >
          <Calendar class="h-16 w-16 text-note-ivory"></Calendar>
        </DashboardNumberCard>
      </div>
      <!-- grid wrapper -->

      <div class="flex flex-1">
        <div class="flex w-full flex-col gap-4 px-[12.5%] py-4">
          <div class="flex items-baseline justify-between">
            <p class="text-xl font-semibold text-note-ivory">Activity heatmap</p>
            <p class="text-xs uppercase tracking-widest text-note-pumice/60">Last 365 days</p>
          </div>
          <!-- month names  -->
          <div class="flex h-4 w-[92%] select-none flex-row justify-evenly text-note-pumice/60">
            <span
              v-for="(label, idx) in monthLabels"
              :key="idx"
              class="h-4 text-[10px] leading-4 text-note-pumice/60"
            >
              {{ label }}
            </span>
          </div>
          <div class="overflow-x-auto pb-2">
            <div class="flex gap-3">
              <!-- day names -->
              <div class="flex flex-col justify-between py-[2px]">
                <span
                  v-for="(label, idx) in weekdayLabels"
                  :key="idx"
                  class="h-4 select-none text-[10px] leading-4 text-note-pumice/60"
                >
                  {{ label }}
                </span>
              </div>

              <!-- heatmap -->
              <div class="inline-grid grid-flow-col grid-rows-7 gap-1">
                <ActivityMapSquare
                  v-for="day in weeks.flat()"
                  :key="day.date"
                  :date="day.date"
                  :numberOfContributions="day.contributions"
                />
              </div>
            </div>
          </div>

          <div class="flex items-center gap-2 text-[11px] text-note-pumice/70">
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
      </div>
      <ScreenDeviderHorizontal />
    </div>
  </div>

  <div class="mt-6 px-[12.5%] pb-10">
    <div class="grid grid-cols-1 gap-6 md:grid-cols-2">
      <!-- Recently edited -->
      <div
        class="flex flex-col gap-3 rounded-2xl border border-note-pumice/20 bg-note-graphite/60 px-5 py-4"
      >
        <div class="flex items-center justify-between">
          <p class="text-lg font-semibold text-note-ivory">Recently edited</p>
          <span class="text-[11px] uppercase tracking-widest text-note-pumice/60">
            last 3 notes
          </span>
        </div>

        <div
          v-if="lastThreeEdited.length"
          class="flex flex-col gap-2"
        >
          <div
            v-for="([noteId, ts], idx) in lastThreeEdited"
            :key="noteId + ts"
            class="flex items-center justify-between rounded-xl border border-note-pumice/15 bg-note-graphite px-3 py-2"
          >
            <div class="flex flex-col">
              <span class="text-sm font-medium text-note-ivory">Note #{{ idx + 1 }}</span>
              <span class="text-xs text-note-pumice/70">
                {{ noteId }}
              </span>
            </div>
            <span class="text-xs text-note-pumice/60">
              {{ new Date(ts).toLocaleString() }}
            </span>
          </div>
        </div>

        <p
          v-else
          class="text-sm text-note-pumice/50"
        >
          No recent edits yet. Start writing your first note.
        </p>
      </div>

      <!-- Favourite tags -->
      <div
        class="flex flex-col gap-3 rounded-2xl border border-note-pumice/20 bg-note-graphite/60 px-5 py-4"
      >
        <div class="flex items-center justify-between">
          <p class="text-lg font-semibold text-note-ivory">Favourite tags</p>
          <span class="text-[11px] uppercase tracking-widest text-note-pumice/60">top 3</span>
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
            <div class="flex items-center gap-2">
              <span
                class="inline-block h-3 w-3 rounded-full"
                :style="{ backgroundColor: color }"
              />
              <span class="text-sm font-medium text-note-ivory">
                {{ tagName }}
              </span>
            </div>
            <span class="text-xs text-note-pumice/60">#{{ idx + 1 }}</span>
          </div>
        </div>

        <p
          v-else
          class="text-sm text-note-pumice/50"
        >
          No tags yet. Add tags to your notes to see them here.
        </p>
      </div>
    </div>
  </div>
</template>

<style scoped></style>
