import { Otlp } from "@effect/opentelemetry";
import { FetchHttpClient } from "@effect/platform";
import { Effect, Layer, Option } from "effect";
import { OpenTelemetrySpanExportUrl, ServiceName, ServiceVersion } from "./configuration.js";

export const makeOpenTelemetryLayer = Effect.gen(function*() {
  const serviceName = yield* ServiceName;
  const serviceVersion = yield* ServiceVersion;
  const otelUrl = yield* OpenTelemetrySpanExportUrl;

  if (Option.isSome(otelUrl)) {
    yield* Effect.log(`OpenTelemetry configuration detected, spans will be sent to ${otelUrl.value}`);

    return Otlp.layerProtobuf({
      resource: { serviceName, serviceVersion },
      baseUrl: String(otelUrl.value),
    }).pipe(Layer.provide(FetchHttpClient.layer));
  }

  return Layer.empty;
});
