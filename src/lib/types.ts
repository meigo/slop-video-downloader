export type DepsStatus = {
  ytdlp: boolean;
  ffmpeg: boolean;
  ytdlp_path: string | null;
  ffmpeg_path: string | null;
};

export type VideoMeta = {
  id: string;
  title: string;
  duration_secs: number;
  thumbnail_url: string | null;
};

export type PreviewResult = {
  mode: "stream" | "file";
  url_or_path: string;
  note?: string | null;
};

/** Video MP4 or audio-only AAC m4a. */
export type ExportKind = "video" | "audio";

export type ExportOpts = {
  url: string;
  title: string;
  start_secs: number;
  end_secs: number;
  max_height: number | null; // null = source; ignored for audio
  include_audio: boolean; // video mux only; ignored for audio (always on)
  out_dir: string;
  export_kind?: ExportKind; // default "video"
};

export type ExportResult = { output_path: string };

export type AppSettings = {
  last_save_dir: string | null;
  max_height: number | null; // null = source
  include_audio: boolean;
};
