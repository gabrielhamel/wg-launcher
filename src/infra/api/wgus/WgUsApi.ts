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
      getMetadata({ gameId, protocolVersion, chainId }: {
        gameId: string;
        protocolVersion: string;
        chainId: string;
      }) {
        return apiClient.get(
          "/metadata",
          GetMetadataResponseSchema,
          {
            guid: gameId,
            protocol_version: protocolVersion,
            chain_id: chainId,
          },
        ).pipe(
          Effect.mapError((error) => new WgUsApiError({ error })),
        );
      },
      getPatchesChains({
        clientType,
        language,
        metadataVersion,
        gameId,
        protocolVersion,
        metadataProtocolVersion,
        patchesChains,
      }: {
        clientType: string;
        language: string;
        metadataVersion: number;
        gameId: string;
        protocolVersion: string;
        metadataProtocolVersion: string;
        patchesChains: {
          id: string;
          version: string;
        }[];
      }) {
        const chainsSearchParams = patchesChains.reduce((accumulator, currentValue) => ({
          ...accumulator,
          [`${currentValue.id}_current_version`]: currentValue.version,
        }), {});

        return apiClient.get("/patches_chain", GetPatchesChainsResponseSchema, {
          client_type: clientType,
          lang: language,
          metadata_version: metadataVersion.toString(),
          game_id: gameId,
          protocol_version: protocolVersion,
          metadata_protocol_version: metadataProtocolVersion,
          ...chainsSearchParams,
        }).pipe(Effect.mapError((error) => new WgUsApiError({ error })));
      },
    };
  }),
  dependencies: [XMLApiClientFactory.Default],
}) {}
