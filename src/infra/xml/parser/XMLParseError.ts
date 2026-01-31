import { Data } from "effect";

type Param = { error: unknown };

export class XMLParseError extends Data.TaggedError("XMLParseError")<Param> {
  constructor(param: Param) {
    super(param);

    this.message = `The XML could not be parsed:\n${param.error}`;
  }
}
