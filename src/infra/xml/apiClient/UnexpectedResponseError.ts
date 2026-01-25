import { Data } from "effect";
import type { ParseError } from "effect/ParseResult";

export class UnexpectedResponseError extends Data.TaggedError("UnexpectedResponseError")<{ parseError: ParseError }> {}
