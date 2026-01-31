import { NodeSdk } from "@effect/opentelemetry";
import { OTLPTraceExporter } from "@opentelemetry/exporter-trace-otlp-grpc";
import { BatchSpanProcessor } from "@opentelemetry/sdk-trace-base";
import { Effect, Layer, Option } from "effect";
import { OpenTelemetrySpanExportGRPCUrl, ServiceName, ServiceVersion } from "./configuration.js";

export const makeOpenTelemetryLayer = Effect.gen(function*() {
  const serviceName = yield* ServiceName;
  const serviceVersion = yield* ServiceVersion;
  const otelUrl = yield* OpenTelemetrySpanExportGRPCUrl;

  if (Option.isSome(otelUrl)) {
    yield* Effect.log(`OpenTelemetry configuration detected, spans will be sent to ${otelUrl.value}`);

    return NodeSdk.layer(() => ({
      resource: { serviceName, serviceVersion },
      spanProcessor: new BatchSpanProcessor(
        new OTLPTraceExporter({
          url: String(otelUrl.value),
        }),
      ),
    }));
  }

  return Layer.empty;
});
