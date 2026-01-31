import { Command } from "@effect/cli";
import { NodeContext, NodeRuntime } from "@effect/platform-node";
import { Config, Console, Effect, Option } from "effect";
import { WgUsApi } from "./infra/api/wgus/WgUsApi.js";
import { makeOpenTelemetryLayer } from "./infra/opentelemetry.js";

const cli = Command.run(
  Command.make("wg-launcher-cli").pipe(
    Command.withSubcommands([Command.make("fetch", {}, (): Effect.Effect<void> =>
      Effect.gen(function*() {
        const wgUsApi = yield* WgUsApi;

        const protocolVersion = "100500.6969696";
        const gameId = "WOT.EU.PRODUCTION";

        const metadata = yield* wgUsApi.getMetadata({
          gameId,
          protocolVersion,
          chainId: "Unknown",
        });

        yield* Console.log(metadata);

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

        yield* Console.log(patchesChains);
      }).pipe(
        Effect.catchTags({
          WgUsApiError: (error) => Effect.die(error),
        }),
        Effect.provide(WgUsApi.Default),
        Effect.withSpan("Command fetch"),
      ))]),
  ),
  {
    name: "Wargaming launcher client",
    version: "0.1.0",
  },
);

const entrypoint = () =>
  Effect.gen(function*() {
    const otelGRPCUrl = yield* Config.url("OPEN_TELEMETRY_EXPORT_GRPC_URL").pipe(Config.option);
    const cliEntryPoint = cli(process.argv).pipe(Effect.withSpan("CLI entrypoint"));

    if (Option.isSome(otelGRPCUrl)) {
      const otellayer = yield* makeOpenTelemetryLayer(otelGRPCUrl.value);
      return yield* cliEntryPoint.pipe(Effect.provide(otellayer));
    }

    return yield* cliEntryPoint;
  });

entrypoint().pipe(Effect.provide(NodeContext.layer), NodeRuntime.runMain);
