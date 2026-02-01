import { Otlp } from "@effect/opentelemetry";
import { FetchHttpClient } from "@effect/platform";
import { Effect, Layer, Option } from "effect";
import { OpenTelemetry, Service } from "./configuration.js";

export const makeOpenTelemetryLayer = Effect.gen(function*() {
  const serviceName = yield* Service.name;
  const serviceVersion = yield* Service.version;
  const exportUrl = yield* OpenTelemetry.exportUrl;

  if (Option.isSome(exportUrl)) {
    yield* Effect.log(`OpenTelemetry configuration detected, spans will be sent to ${exportUrl.value}`);

    const headerKey = yield* OpenTelemetry.headers.key;
    const headerValue = yield* OpenTelemetry.headers.value;

    const headers = (Option.isSome(headerKey) && Option.isSome(headerValue))
      ? { [headerKey.value]: headerValue.value }
      : {};

    return Otlp.layerProtobuf({
      resource: { serviceName, serviceVersion },
      baseUrl: String(exportUrl.value),
      headers,
    }).pipe(Layer.provide(FetchHttpClient.layer));
  }

  return Layer.empty;
});
