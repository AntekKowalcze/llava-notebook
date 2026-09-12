import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { useToast } from 'vue-toastification';

export async function checkForUpdates() {
  const toast = useToast();

  try {
    const update = await check();

    if (!update) {
      return;
    }

    toast.success(
      `Update ${update.version} found. It will be installed automatically.`
    );

    await update.downloadAndInstall();
    await relaunch();
  } catch (error) {
    console.error('Update check failed', error);
  }
}