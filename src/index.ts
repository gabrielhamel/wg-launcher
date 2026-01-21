import { Command } from "@effect/cli";
import { NodeContext, NodeRuntime } from "@effect/platform-node";
import { Console, Effect } from "effect";

const entrypoint = () => {
  Console.log("Hello World");

  return Effect.succeed(Effect.void);
};

const cli = Command.run(Command.make("wg-launcher-cli", {}, entrypoint), {
  name: "Wargaming launcher client",
  version: "0.1.0",
});

cli(process.argv).pipe(Effect.provide(NodeContext.layer), NodeRuntime.runMain);
