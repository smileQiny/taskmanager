export interface ReleaseAsset {
  name: string;
  downloadUrl: string;
  size: number;
  contentType?: string;
}

export interface AppUpdateInfo {
  currentVersion: string;
  latestVersion: string;
  releaseName: string;
  releaseUrl: string;
  publishedAt: string | null;
  assets: ReleaseAsset[];
  preferredAsset: ReleaseAsset | null;
  isUpdateAvailable: boolean;
  checkedAt: string;
}

export type UpdateCheckSource = 'startup' | 'manual';

export type UpdateStatus = 'idle' | 'checking' | 'update_available' | 'up_to_date' | 'error';
