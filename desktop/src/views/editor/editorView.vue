<script setup lang="ts">
import { MilkdownProvider } from '@milkdown/vue'
import '@milkdown/crepe/theme/common/style.css'
import LeftEditorPanel from '../../components/editor/LeftEditorPanel.vue';
import { ArrowBigLeftDash } from 'lucide-vue-next';
import { useRouter } from 'vue-router';
const router = useRouter()
//  import '@milkdown/crepe/theme/nord-dark.css'
import '../../css/milkdown.css'
import MilkdownEditor from '../../components/editor/MilkdownEditor.vue';
function redirect() {
    console.log("clicked")
    router.push({name:"dashboard"});
            console.log("after:", router.currentRoute.value.fullPath);
    }
</script>
<template>
    <ArrowBigLeftDash
        class="active:scale-90  absolute left-[2%] top-[93%] text-note-paprika/80 transition-transform duration-200 hover:scale-95"
        @click="redirect"></ArrowBigLeftDash>
    <div class="flex flex-1">
        <!-- <LeftEditorPanel></LeftEditorPanel> -->
        <div class="flex flex-1 min-h-0 overflow-y-auto">
            <MilkdownProvider>
                <MilkdownEditor />
            </MilkdownProvider>
        </div>
    </div>
</template>

