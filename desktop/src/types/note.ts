export type UUID = string;
export interface Note {
  local_id: UUID;
  owner_id: UUID;
  name: string;
  title: string;
  summary: string;
  created_at: number;
  updated_at: number;
  is_deleted: boolean;
  deleted_at: number | undefined;
}
