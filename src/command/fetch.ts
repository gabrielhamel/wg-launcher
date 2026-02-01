import { Effect } from "effect";
import { WgUsApi } from "../infra/api/wgus/WgUsApi.js";

export const fetchCommandHandler: Effect.Effect<void> = Effect.gen(function*() {
  const wgUsApi = yield* WgUsApi;

  const protocolVersion = "100500.6969696";
  const gameId = "WOT.EU.PRODUCTION";

  const metadata = yield* wgUsApi.getMetadata({
    gameId,
    protocolVersion,
    chainId: "Unknown",
  });

  yield* Effect.log(metadata);

  const patchesChains = yield* wgUsApi.getPatchesChains({
    gameId,
    protocolVersion,
    language: metadata.protocol.predefined_section.default_language,
    metadataVersion: metadata.protocol.version,
    metadataProtocolVersion: protocolVersion,
    clientType: "sd",
    patchesChains: [
      { id: "sdcontent", version: "0" },
      { id: "locale", version: "0" },
      { id: "client", version: "0" },
      { id: "hdcontent", version: "0" },
    ],
  });

  yield* Effect.log(patchesChains);
}).pipe(
  Effect.catchTags({
    WgUsApiError: (error) => Effect.die(error),
  }),
  Effect.provide(WgUsApi.Default),
  Effect.withSpan("Command fetch"),
);
