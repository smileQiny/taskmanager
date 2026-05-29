import { create } from 'zustand';
import {
  appVersion,
  checkForUpdates,
  openExternalUrl,
  releasesUrl,
} from '../services/updateService';
import { AppUpdateInfo, UpdateCheckSource, UpdateStatus } from '../types/update';

interface UpdateStore {
  currentVersion: string;
  release: AppUpdateInfo | null;
  status: UpdateStatus;
  error: string | null;
  startupChecked: boolean;
  check: (source?: UpdateCheckSource) => Promise<AppUpdateInfo | null>;
  openReleasePage: () => Promise<void>;
  openPreferredDownload: () => Promise<void>;
}

export const useUpdateStore = create<UpdateStore>((set, get) => ({
  currentVersion: appVersion,
  release: null,
  status: 'idle',
  error: null,
  startupChecked: false,

  check: async (source = 'manual') => {
    const current = get();
    if (source === 'startup' && current.startupChecked) return current.release;
    if (current.status === 'checking') return current.release;

    set({
      status: 'checking',
      error: null,
      startupChecked: source === 'startup' ? true : current.startupChecked,
    });

    try {
      const release = await checkForUpdates({ currentVersion: get().currentVersion });
      set({
        release,
        status: release.isUpdateAvailable ? 'update_available' : 'up_to_date',
        error: null,
      });
      return release;
    } catch (error) {
      set({
        status: 'error',
        error: error instanceof Error ? error.message : String(error),
      });
      return null;
    }
  },

  openReleasePage: async () => {
    await openExternalUrl(get().release?.releaseUrl ?? releasesUrl);
  },

  openPreferredDownload: async () => {
    const release = get().release;
    await openExternalUrl(release?.preferredAsset?.downloadUrl ?? release?.releaseUrl ?? releasesUrl);
  },
}));
