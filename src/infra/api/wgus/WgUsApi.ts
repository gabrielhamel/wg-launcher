import { Effect } from "effect";
import { XMLApiClientFactory } from "../../xml/apiClient/XMLApiClientFactory.js";
import { GetMetadataResponseSchema } from "./schemas/GetMetadataResponseSchema.js";
import { GetPatchesChainsResponseSchema } from "./schemas/GetPatchesChainsResponseSchema.js";
import { WgUsApiError } from "./WgUsApiError.js";

export class WgUsApi extends Effect.Service<WgUsApi>()("WgUsApi", {
  effect: Effect.gen(function*() {
    const XmlApiClientFactory = yield* XMLApiClientFactory;

    const apiClient = XmlApiClientFactory.make({
      baseUrl: "https://wgus-woteu.wargaming.net/api/v1",
    });

    return {
      getMetadata: () =>
        apiClient.get(
          "/metadata",
          GetMetadataResponseSchema,
          {
            guid: "WOT.EU.PRODUCTION",
            protocol_version: "100500.6969696",
            chain_id: "unknown",
          },
        ).pipe(Effect.catchTags({
          ApiResponseError: (technicalError) => Effect.fail(new WgUsApiError({ technicalError })),
          ResponseError: (technicalError) => Effect.fail(new WgUsApiError({ technicalError })),
          UnexpectedResponseError: (technicalError) => Effect.fail(new WgUsApiError({ technicalError })),
          RequestError: (technicalError) => Effect.fail(new WgUsApiError({ technicalError })),
          XMLParseError: (technicalError) => Effect.fail(new WgUsApiError({ technicalError })),
        })),
      getPatchesChains: () =>
        apiClient.get("/patches_chain", GetPatchesChainsResponseSchema, {
          client_type: "sd",
          lang: "FR",
          metadata_version: "20251013145823",
          game_id: "WOT.EU.PRODUCTION",
          protocol_version: "100500.6969696",
          metadata_protocol_version: "100500.6969696",
          sdcontent_current_version: "0",
          locale_current_version: "0",
          client_current_version: "0",
          hdcontent_current_version: "0",
        }).pipe(Effect.catchTags({
          ApiResponseError: (technicalError) => Effect.fail(new WgUsApiError({ technicalError })),
          ResponseError: (technicalError) => Effect.fail(new WgUsApiError({ technicalError })),
          UnexpectedResponseError: (technicalError) => Effect.fail(new WgUsApiError({ technicalError })),
          RequestError: (technicalError) => Effect.fail(new WgUsApiError({ technicalError })),
          XMLParseError: (technicalError) => Effect.fail(new WgUsApiError({ technicalError })),
        })),
    };
  }),
  dependencies: [XMLApiClientFactory.Default],
}) {}
