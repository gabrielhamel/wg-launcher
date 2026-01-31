import { Data } from "effect";
import type { ParseError } from "effect/ParseResult";

type Param = { error: ParseError };

export class UnexpectedResponseError extends Data.TaggedError("UnexpectedResponseError")<Param> {
  constructor(param: Param) {
    super(param);

    this.message = `The response from the API was not as expected:\n${param.error}`;
  }
}
