import { Config } from "effect";

export const ServiceName = Config.nonEmptyString("RELEASE_NAME").pipe(Config.withDefault("wg-launcher-cli"));

export const ServiceVersion = Config.nonEmptyString("RELEASE_VERSION").pipe(Config.withDefault("unknown"));

export const OpenTelemetrySpanExportUrl = Config.url("OPEN_TELEMETRY_SPAN_EXPORT_URL").pipe(Config.option);
