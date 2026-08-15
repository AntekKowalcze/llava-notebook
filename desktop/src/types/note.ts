export interface Note {
  local_id: string;
  mongo_id: string | null;
  owner_id: string;

  title: string;
  summary: string;
  content_path: string;

  created_at: number;
  updated_at: number;
  is_deleted: boolean;
  deleted_at: number | null;

  version: number;
  cloud_version: number | null;
  sync_state: SyncState;

  encrypted: boolean;

  crypto_meta: any | null;
}

export type SyncState =
  | 'Synced'
  | 'PendingUpload'
  | 'PendingDownload'
  | 'Conflict'
  | 'Error';