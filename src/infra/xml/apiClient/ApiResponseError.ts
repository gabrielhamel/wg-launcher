import { Data } from "effect";

export class ApiResponseError extends Data.TaggedError("ApiResponseError")<{ body: string }> {}
