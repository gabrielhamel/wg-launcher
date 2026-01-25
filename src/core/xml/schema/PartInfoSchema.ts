import { Schema } from "effect";

export const PartSchema = Schema.Struct({
  name: Schema.String,
  version: Schema.Number,
  status: Schema.String,
});

export const PartsInfoSchema = Schema.Struct({
  part: Schema.Array(PartSchema),
});

export const UrlsSchema = Schema.Struct({
  url: Schema.String,
});

export const TorrentSchema = Schema.Struct({
  hash: Schema.String,
  urls: UrlsSchema,
});

export const FilesSchema = Schema.Struct({
  file: Schema.Unknown,
});

export const PatchSchema = Schema.Struct({
  files: FilesSchema,
  torrent: TorrentSchema,
  part: Schema.String,
  version_from: Schema.Unknown,
  version_to: Schema.String,
});

export const PatchesChainSchema = Schema.Struct({
  patch: Schema.Array(PatchSchema),
});

export const WebSeedsSchema = Schema.Struct({
  url: Schema.String,
});

export const ProtocolSchema = Schema.Struct({
  parts_info: PartsInfoSchema,
  patches_chain: PatchesChainSchema,
  delay_preload: Schema.String,
  web_seeds: WebSeedsSchema,
  meta_need_update: Schema.String,
  version_name: Schema.String,
});

export const RootSchema = Schema.Struct({
  protocol: ProtocolSchema,
});

export type Root = typeof RootSchema.Type;
export type Protocol = typeof ProtocolSchema.Type;
export type PartsInfo = typeof PartsInfoSchema.Type;
export type Part = typeof PartSchema.Type;
export type PatchesChain = typeof PatchesChainSchema.Type;
export type Patch = typeof PatchSchema.Type;
export type Files = typeof FilesSchema.Type;
export type Torrent = typeof TorrentSchema.Type;
export type Urls = typeof UrlsSchema.Type;
export type WebSeeds = typeof WebSeedsSchema.Type;
