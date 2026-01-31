import { Data } from "effect";

type Param = { body: string; status: number };

export class ApiResponseError extends Data.TaggedError("ApiResponseError")<Param> {
  constructor(param: Param) {
    super(param);

    this.message = `The API responded with status code ${param.status}:\n${param.body}`;
  }
}
