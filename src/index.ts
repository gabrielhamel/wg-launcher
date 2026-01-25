import { Command } from "@effect/cli";
import { NodeContext, NodeRuntime } from "@effect/platform-node";
import { Console, Effect } from "effect";
import { WgUsApi } from "./infra/api/wgus/WgUsApi.js";

const cli = Command.run(
  Command.make("wg-launcher-cli").pipe(
    Command.withSubcommands([Command.make("fetch", {}, (): Effect.Effect<void> =>
      Effect.gen(function*() {
        const wgUsApi = yield* WgUsApi;
        const metadata = yield* wgUsApi.getPatchesChains();

        yield* Console.log(metadata);
      }).pipe(
        Effect.catchTags({
          WgUsApiError: (error) => Console.error(`WgUsApiError: ${JSON.stringify(error)}`),
        }),
        Effect.provide(WgUsApi.Default),
      ))]),
  ),
  {
    name: "Wargaming launcher client",
    version: "0.1.0",
  },
);

cli(process.argv).pipe(Effect.provide(NodeContext.layer), NodeRuntime.runMain);
