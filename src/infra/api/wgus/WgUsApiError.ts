import type { RequestError, ResponseError } from "@effect/platform/HttpClientError";
import { Data } from "effect";
import type { ApiResponseError } from "../../xml/apiClient/ApiResponseError.js";
import type { UnexpectedResponseError } from "../../xml/apiClient/UnexpectedResponseError.js";
import type { XMLParseError } from "../../xml/parser/XMLParseError.js";

export class WgUsApiError extends Data.TaggedError("WgUsApiError")<
  { technicalError: ResponseError | RequestError | UnexpectedResponseError | XMLParseError | ApiResponseError }
> {}
