<script setup lang="ts">
import SubmitButton from "../components/commons/SubmitButton.vue";
import IconComponent from "../components/main/IconComponent.vue";
import { LockKeyhole, Cloud } from "lucide-vue-next";
import SwitchInput from "../components/settings/SwitchInput.vue";
import TextInput from "../components/auth/forms/TextInput.vue";
import { InputTypes } from "../types/inputTypes";
import { ref } from "vue";
import { useToast } from "vue-toastification";
import { invoke } from "@tauri-apps/api/core";
import { Note } from "../types/note.ts";
import { useCurrentNoteStore } from "../stores/currentNoteStore.ts";
import { useRouter } from "vue-router";
const router = useRouter()
const toast = useToast()
const currentNoteStore = useCurrentNoteStore();
const sync = ref<string>("")
const encryption = ref<string>("")
const title = ref<string>("")
    
    function settingChanged(id: string, value: string) {
    if (id === "sync") {
        sync.value = value
    }
    if (id === "encryption") {
        encryption.value = value
    }
}
function getErrorText(err: unknown): string {
  if (typeof err === "string") {
    return err
  }

  if (err && typeof err === "object") {
    const typedErr = err as {
      message?: unknown
      error?: unknown
      reason?: unknown
    }

    if (typeof typedErr.message === "string") {
      return typedErr.message
    }

    if (typeof typedErr.error === "string") {
      return typedErr.error
    }

    if (typeof typedErr.reason === "string") {
      return typedErr.reason
    }
  }

  return String(err ?? "")
}

async function createNote(): Promise<void> {
  if (title.value.trim().length === 0) {
    toast.warning("Title cannot be empty")
    return
  }

  const useEncryption = encryption.value === "on"
  console.log("TO JEST TO", useEncryption + encryption.value)
  const useSynchronization = sync.value === "on"

  try {
    const createdNote = await invoke<Note>("create_note", {
      title: title.value.trim(),
      encryption: useEncryption,
      synchronizing: useSynchronization,
    })

    currentNoteStore.$patch({
      currentNote: createdNote,
    })

    toast.success("Note created successfully")

    await router.push(`/main/editor/${createdNote.local_id}`)
  } catch (err: unknown) {
    console.error("Failed to create note:", err)

    const message = getErrorText(err).toLowerCase()

    if (message.includes("note name already exists")) {
      toast.warning("A note with this name already exists.")
    } else if (message.includes("note name after sanitization is empty")) {
      toast.warning("The note title is invalid.")
    } else if (message.includes("title too long")) {
      toast.warning("The note title is too long.")
    } else if (message.includes("name too long")) {
      toast.warning("The note name is too long.")
    } else if (message.includes("encryption key is unavailable")) {
      toast.error("Encryption key is unavailable.")
    } else if (message.includes("file operation error")) {
      toast.error("Failed to create the note file.")
    } else if (message.includes("couldn't lock state")) {
      toast.error("Failed to access application state.")
    } else if (message.includes("internal error")) {
      toast.error("An internal error occurred while creating the note.")
    } else {
      toast.error("Failed to create note.")
    }
  }
}

</script>

<template>
   
    <div class="flex h-full w-full items-center justify-center bg-note-graphite px-8">
        <div class="w-full max-w-4xl">

            <!-- Icon -->
            <div class="flex justify-center">

                <IconComponent width="w-56" height="h-56" />

            </div>


            <!-- Heading -->
            <div class="mt-10 text-center">


                <h1 class="
          mx-auto
          mt-6
          max-w-2xl
          text-4xl
          font-bold
          leading-relaxed
          text-note-pumice
          ">
                    <span class="text-note-paprika">
                        Your ideas
                    </span>
                    deserve a place to stay.
                    <br />
                    Give them a home.
                </h1>

            </div>


            <div class="
        mt-14
        rounded-3xl
        border border-note-pumice/10
        bg-black/40
        backdrop-blur-2xl
        p-10
        shadow-2xl
        ">

                <div>
                    <label class="
            mb-3
            block
            text-sm
            uppercase
            tracking-[0.25em]
            text-note-pumice/50
            ">
                        Note title
                    </label>

                    <TextInput name="" placeholder="Give your note a title" :type="InputTypes.Text" class="w-full" v-model="title">
                    </TextInput>

                </div>


                <!-- Settings -->
                <div class="mt-10 space-y-4">


                    <div class="
            flex
            items-center
            justify-between
            rounded-2xl
            border border-note-pumice/10
            bg-black/30
            px-6
            py-5
            ">

                        <div class="flex items-center gap-5">

                            <div class="
                flex
                h-14
                w-14
                items-center
                justify-center
                rounded-xl
                bg-note-paprika/10
                ">
                                <LockKeyhole class="h-7 w-7 text-note-paprika" />
                            </div>


                            <div>
                                <p class="
                  text-lg
                  text-note-ivory
                  ">
                                    Encryption
                                </p>

                                <p class="
                  text-sm
                  text-note-pumice/50
                  ">
                                    Protect your private thoughts
                                </p>
                            </div>

                        </div>
<SwitchInput
    :current-value="sync"
    id="sync"
    @setting-changed="settingChanged"
/>



                    </div>



                    <div class="
            flex
            items-center
            justify-between
            rounded-2xl
            border border-note-pumice/10
            bg-black/30
            px-6
            py-5
            ">

                        <div class="flex items-center gap-5">

                            <div class="
                flex
                h-14
                w-14
                items-center
                justify-center
                rounded-xl
                bg-note-glow/10
                ">
                                <Cloud class="h-7 w-7 text-note-glow" />
                            </div>


                            <div>

                                <p class="
                  text-lg
                  text-note-ivory
                  ">
                                    Synchronization
                                </p>

                                <p class="
                  text-sm
                  text-note-pumice/50
                  ">
                                    Keep your knowledge everywhere
                                </p>

                            </div>

                        </div>


<SwitchInput
    :current-value="encryption"
    id="encryption"
    @setting-changed="settingChanged"
/>
                    </div>

                </div>

<SubmitButton
    content="Create note"
    class="mt-8 w-full h-14 text-xl active:scale-[98%]"
    @click="createNote"
/>
               


            </div>

        </div>
    </div>
</template>