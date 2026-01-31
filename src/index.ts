import { Command } from "@effect/cli";
import { NodeContext, NodeRuntime } from "@effect/platform-node";
import { Config, Effect } from "effect";
import { fetchCommandHandler } from "./core/command/fetch.js";
import { OpenTelemetryLive } from "./infra/opentelemetry.js";

const buildCLI = ({ name, version }: {
  name: string;
  version: string;
}) =>
  Command.run(
    Command.make(name).pipe(
      Command.withSubcommands([Command.make("fetch", {}, () => fetchCommandHandler)]),
    ),
    {
      name,
      version,
    },
  );

const entrypoint = () =>
  Effect.gen(function*() {
    const serviceName = yield* Config.nonEmptyString("RELEASE_NAME").pipe(Config.withDefault("unknown"));
    const serviceVersion = yield* Config.nonEmptyString("RELEASE_VERSION").pipe(Config.withDefault("unknown"));

    const runCLI = buildCLI({ name: serviceName, version: serviceVersion })(process.argv).pipe(
      Effect.withSpan("CLI entrypoint"),
    );

    return yield* runCLI.pipe(Effect.provide(OpenTelemetryLive));
  });

entrypoint().pipe(Effect.provide(NodeContext.layer), NodeRuntime.runMain);
