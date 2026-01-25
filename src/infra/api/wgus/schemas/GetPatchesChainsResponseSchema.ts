import { Schema } from "effect";

const PartSchema = Schema.Struct({
  name: Schema.String,
  version: Schema.Number,
  status: Schema.String,
});

const PartsInfoSchema = Schema.Struct({
  part: Schema.Array(PartSchema),
});

const UrlsSchema = Schema.Struct({
  url: Schema.String,
});

const TorrentSchema = Schema.Struct({
  hash: Schema.String,
  urls: UrlsSchema,
});

const FilesSchema = Schema.Struct({
  file: Schema.Unknown,
});

const PatchSchema = Schema.Struct({
  files: FilesSchema,
  torrent: TorrentSchema,
  part: Schema.String,
  version_from: Schema.Unknown,
  version_to: Schema.String,
});

const PatchesChainSchema = Schema.Struct({
  patch: Schema.Array(PatchSchema),
});

const WebSeedsSchema = Schema.Struct({
  url: Schema.String,
});

const ProtocolSchema = Schema.Struct({
  parts_info: PartsInfoSchema,
  patches_chain: PatchesChainSchema,
  delay_preload: Schema.String,
  web_seeds: WebSeedsSchema,
  meta_need_update: Schema.String,
  version_name: Schema.String,
});

export const GetPatchesChainsResponseSchema = Schema.Struct({
  protocol: ProtocolSchema,
});
