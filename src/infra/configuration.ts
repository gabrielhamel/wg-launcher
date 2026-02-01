import { Config } from "effect";

export const Service = {
  name: Config.nonEmptyString("RELEASE_NAME").pipe(Config.withDefault("wg-launcher-cli")),
  version: Config.nonEmptyString("RELEASE_VERSION").pipe(Config.withDefault("unknown")),
};

export const OpenTelemetry = {
  exportUrl: Config.url("OPEN_TELEMETRY_EXPORT_URL").pipe(Config.option),
  headers: {
    key: Config.nonEmptyString("OPEN_TELEMETRY_EXPORT_HEADER_KEY").pipe(Config.option),
    value: Config.nonEmptyString("OPEN_TELEMETRY_EXPORT_HEADER_VALUE").pipe(Config.option),
  },
};
