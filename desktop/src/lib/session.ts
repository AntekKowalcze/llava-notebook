import { useAuthStore } from '../stores/auth';
import { useOnlineAuthStore } from '../stores/onlineAuth';
import { useUserConfigStore } from '../stores/userConfig';
import { useCurrentNoteStore } from '../stores/currentNoteStore';
import { useLayoutStore } from '../stores/layoutStore';

/**
 * Forgets everything the UI holds about the signed-in local user. Used after
 * `local_logout_command` so the next user does not briefly see the previous
 * user's open note, word count or panels.
 */
export function resetSessionState() {
  useAuthStore().$patch({
    loggedIn: false,
    loggedInUsername: null,
    loggedInUserId: null,
  });
  useOnlineAuthStore().$patch({
    loggedIn: false,
    loggedInEmail: null,
    loggedInId: null,
  });
  useUserConfigStore().settingList = null;

  const currentNoteStore = useCurrentNoteStore();
  currentNoteStore.currentNote = undefined;
  currentNoteStore.words = 0;

  const layout = useLayoutStore();
  layout.closeTagEditor();
  layout.leftPanelOpen = false;
}
