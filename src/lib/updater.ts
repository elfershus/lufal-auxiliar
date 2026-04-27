import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

export type { Update };

export async function checkForUpdates(): Promise<Update | null> {
  try {
    const update = await check();
    return update ?? null;
  } catch {
    return null;
  }
}

export async function installUpdate(
  update: Update,
  onProgress: (downloaded: number, total: number | null) => void
): Promise<void> {
  let downloaded = 0;
  let total: number | null = null;

  await update.downloadAndInstall((event) => {
    if (event.event === 'Started') {
      total = event.data.contentLength ?? null;
    } else if (event.event === 'Progress') {
      downloaded += event.data.chunkLength;
      onProgress(downloaded, total);
    }
  });

  await relaunch();
}
