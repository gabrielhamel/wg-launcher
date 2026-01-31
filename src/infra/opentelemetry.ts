import { NodeSdk } from "@effect/opentelemetry";
import { OTLPTraceExporter } from "@opentelemetry/exporter-trace-otlp-grpc";
import { BatchSpanProcessor } from "@opentelemetry/sdk-trace-base";
import { Config, Context, Effect, Layer, Option } from "effect";

export class OpenTelemetry extends Context.Tag("OpenTelemetry")<OpenTelemetry, void>() {}

export const OpenTelemetryLive = Layer.effect(
  OpenTelemetry,
  Effect.gen(function*() {
    const serviceName = yield* Config.nonEmptyString("RELEASE_NAME").pipe(Config.withDefault("unknown"));
    const serviceVersion = yield* Config.nonEmptyString("RELEASE_VERSION").pipe(Config.withDefault("unknown"));
    const otelUrl = yield* Config.url("OPEN_TELEMETRY_EXPORT_GRPC_URL").pipe(Config.option);

    if (Option.isSome(otelUrl)) {
      yield* Effect.log(`OpenTelemetry detected, span will be sent to ${otelUrl}`);

      return NodeSdk.layer(() => ({
        resource: { serviceName, serviceVersion },
        spanProcessor: new BatchSpanProcessor(
          new OTLPTraceExporter({
            url: String(otelUrl),
          }),
        ),
      }));
    }

    return Layer.empty;
  }),
);
