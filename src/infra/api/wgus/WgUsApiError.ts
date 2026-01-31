import type { RequestError, ResponseError } from "@effect/platform/HttpClientError";
import { Data } from "effect";
import type { ApiResponseError } from "../../xml/apiClient/ApiResponseError.js";
import type { UnexpectedResponseError } from "../../xml/apiClient/UnexpectedResponseError.js";
import type { XMLParseError } from "../../xml/parser/XMLParseError.js";

type Param = {
  error: ResponseError | RequestError | UnexpectedResponseError | XMLParseError | ApiResponseError;
};

export class WgUsApiError extends Data.TaggedError("WgUsApiError")<
  Param
> {
  constructor(
    param: Param,
  ) {
    super(param);

    this.message = `The request to the Wargaming Update Service failed:\n${param.error}`;
  }
}
