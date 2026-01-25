import { Data } from "effect";

export class XMLParseError extends Data.TaggedError("XMLParseError")<{ internalError: unknown }> {}
