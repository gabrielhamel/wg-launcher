import { FetchHttpClient, HttpClient } from "@effect/platform";
import { Effect, Schema } from "effect";
import { XMLParser } from "../parser/XMLParser.js";
import { ApiResponseError } from "./ApiResponseError.js";
import { UnexpectedResponseError } from "./UnexpectedResponseError.js";

export class XMLApiClientFactory extends Effect.Service<XMLApiClientFactory>()("XMLApiClientFactory", {
  effect: Effect.gen(function*() {
    const httpClient = yield* HttpClient.HttpClient;
    const xmlParser = yield* XMLParser;

    return {
      make: ({ baseUrl }: { baseUrl: string }) => ({
        get: <ResponseBody extends object>(
          path: string,
          responseSchema: Schema.Schema<ResponseBody>,
          searchParams: Record<string, string>,
        ) =>
          Effect.gen(function*() {
            const result = yield* httpClient.get(
              `${baseUrl}${path}`,
              {
                urlParams: searchParams,
              },
            );
            const text = yield* result.text;
            const xml = yield* xmlParser.parse(text);

            if (result.status !== 200) {
              return yield* new ApiResponseError({
                body: text,
                status: result.status,
              });
            }

            return yield* Schema.decodeUnknown(responseSchema)(xml).pipe(
              Effect.catchTag("ParseError", (error) => Effect.fail(new UnexpectedResponseError({ error }))),
            );
          }).pipe(Effect.withSpan(`XML GET ${path}`)),
      }),
    };
  }),
  dependencies: [FetchHttpClient.layer, XMLParser.Default],
}) {}
