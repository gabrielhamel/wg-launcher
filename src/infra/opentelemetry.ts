import { NodeSdk } from "@effect/opentelemetry";
import { OTLPTraceExporter } from "@opentelemetry/exporter-trace-otlp-grpc";
import { BatchSpanProcessor } from "@opentelemetry/sdk-trace-base";
import { Effect } from "effect";

export const createOpenTelemetryLayer = (
  { spanExportGRPCUrl, serviceVersion, serviceName }: {
    spanExportGRPCUrl: URL;
    serviceName: string;
    serviceVersion: string;
  },
) =>
  Effect.gen(function*() {
    yield* Effect.log(`OpenTelemetry detected, span will be sent to ${spanExportGRPCUrl}`);

    return NodeSdk.layer(() => ({
      resource: { serviceName, serviceVersion },
      spanProcessor: new BatchSpanProcessor(
        new OTLPTraceExporter({
          url: String(spanExportGRPCUrl),
        }),
      ),
    }));
  });
