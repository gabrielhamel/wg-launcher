import { Command } from "@effect/cli";
import { NodeContext, NodeRuntime } from "@effect/platform-node";
import { Effect } from "effect";
import { fetchCommandHandler } from "./command/fetch.js";
import { Service } from "./infra/configuration.js";
import { makeOpenTelemetryLayer } from "./infra/openTelemetry.js";

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
    const serviceName = yield* Service.name;
    const serviceVersion = yield* Service.version;

    const runCLI = buildCLI({ name: serviceName, version: serviceVersion })(process.argv).pipe(
      Effect.withSpan("CLI entrypoint"),
    );
    const openTelemetry = yield* makeOpenTelemetryLayer;

    return yield* runCLI.pipe(Effect.provide(openTelemetry));
  });

entrypoint().pipe(
  Effect.provide(NodeContext.layer),
  NodeRuntime.runMain,
);
