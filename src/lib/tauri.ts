import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  DepsStatus,
  ExportOpts,
  ExportResult,
  PreviewResult,
  VideoMeta,
} from "./types";

export const checkDeps = () => invoke<DepsStatus>("check_deps");
export const fetchMetadata = (url: string) => invoke<VideoMeta>("fetch_metadata", { url });
export const resolvePreview = (url: string) => invoke<PreviewResult>("resolve_preview", { url });
export const exportClip = (opts: ExportOpts) => invoke<ExportResult>("export_clip", { opts });
export const loadSettings = () => invoke<AppSettings>("load_settings");
export const saveSettings = (settings: AppSettings) =>
  invoke<void>("save_settings", { settings });
export const defaultSaveDir = () => invoke<string>("default_save_dir");
