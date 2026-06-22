import { create } from 'zustand';
import {
  appVersion,
  checkForUpdates,
  installUpdateFromRelease,
  openExternalUrl,
  releasesUrl,
} from '../services/updateService';
import { AppUpdateInfo, InstallUpdateResult, UpdateCheckSource, UpdateStatus } from '../types/update';

interface UpdateStore {
  currentVersion: string;
  release: AppUpdateInfo | null;
  installResult: InstallUpdateResult | null;
  status: UpdateStatus;
  error: string | null;
  startupChecked: boolean;
  check: (source?: UpdateCheckSource) => Promise<AppUpdateInfo | null>;
  openReleasePage: () => Promise<void>;
  installLatestUpdate: () => Promise<InstallUpdateResult | null>;
}

export const useUpdateStore = create<UpdateStore>((set, get) => ({
  currentVersion: appVersion,
  release: null,
  installResult: null,
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
      installResult: null,
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

  installLatestUpdate: async () => {
    const release = get().release;
    if (!release?.isUpdateAvailable) return null;

    set({
      status: 'installing',
      error: null,
      installResult: null,
    });

    try {
      const result = await installUpdateFromRelease(release);
      set({
        status: result.action === 'up_to_date' ? 'up_to_date' : 'update_handled',
        installResult: result,
        error: null,
      });
      return result;
    } catch (error) {
      set({
        status: 'error',
        error: error instanceof Error ? error.message : String(error),
      });
      return null;
    }
  },
}));
