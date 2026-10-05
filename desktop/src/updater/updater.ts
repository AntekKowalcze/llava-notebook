import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { useToast } from 'vue-toastification';

/**
 * Looks for an update and asks before installing it. Installing restarts the
 * app, which must not happen while the user is in the middle of writing.
 */
export async function checkForUpdates() {
  const toast = useToast();

  try {
    const update = await check();

    if (!update) {
      return;
    }

    toast.info(`Update ${update.version} is available. Click here to install it and restart.`, {
      timeout: false,
      closeOnClick: true,
      onClick: async () => {
        try {
          toast.info('Downloading update...');
          await update.downloadAndInstall();
          await relaunch();
        } catch (error) {
          console.error('Update installation failed', error);
          toast.error('Update failed. Try again later.');
        }
      },
    });
  } catch (error) {
    console.error('Update check failed', error);
  }
}
