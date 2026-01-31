import { NodeSdk } from "@effect/opentelemetry";
import { OTLPTraceExporter } from "@opentelemetry/exporter-trace-otlp-grpc";
import { BatchSpanProcessor } from "@opentelemetry/sdk-trace-base";
import { Effect } from "effect";

export const makeOpenTelemetryLayer = (grpcUrl: URL) => {
  return Effect.gen(function*() {
    yield* Effect.log(`OpenTelemetry detected, span will be sent to ${grpcUrl}`);

    return NodeSdk.layer(() => ({
      resource: { serviceName: "wg-launcher-cli", serviceVersion: "0.1.0" },
      spanProcessor: new BatchSpanProcessor(
        new OTLPTraceExporter({
          url: String(grpcUrl),
        }),
      ),
    }));
  });
};
