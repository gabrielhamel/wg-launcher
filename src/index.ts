import { Command } from "@effect/cli";
import { NodeContext, NodeRuntime } from "@effect/platform-node";
import { Config, Effect, Option } from "effect";
import { fetchCommandHandler } from "./core/command/fetch.js";
import { createOpenTelemetryLayer } from "./infra/opentelemetry.js";

const makeCLI = ({ name, version }: {
  name: string;
  version: string;
}) =>
  Command.run(
    Command.make("wg-launcher-cli").pipe(
      Command.withSubcommands([Command.make("fetch", {}, () => fetchCommandHandler)]),
    ),
    {
      name,
      version,
    },
  );

const setup = ({ serviceName }: { serviceName: string }) =>
  Effect.gen(function*() {
    const serviceVersion = yield* Config.nonEmptyString("RELEASE_VERSION").pipe(Config.withDefault("unknown"));
    const otelUrl = yield* Config.url("OPEN_TELEMETRY_EXPORT_GRPC_URL").pipe(Config.option);
    const cliEntryPoint = makeCLI({ name: serviceName, version: serviceVersion })(process.argv).pipe(
      Effect.withSpan("CLI entrypoint"),
    );

    if (Option.isSome(otelUrl)) {
      const otelLayer = yield* createOpenTelemetryLayer({
        serviceName,
        serviceVersion,
        spanExportGRPCUrl: otelUrl.value,
      });
      return yield* cliEntryPoint.pipe(Effect.provide(otelLayer));
    }

    return yield* cliEntryPoint;
  });

setup({
  serviceName: "Wargaming launcher client",
}).pipe(Effect.provide(NodeContext.layer), NodeRuntime.runMain);
