import { Effect } from "effect";
import { XMLParser as FastXMLParser } from "fast-xml-parser";
import { XMLParseError } from "./XMLParseError.js";

export class XMLParser extends Effect.Service<XMLParser>()("XMLParser", {
  sync: () => {
    const parser = new FastXMLParser();

    return {
      parse: (textContent: string) => {
        return Effect.try({
          try: () => parser.parse(textContent),
          catch: (error) => new XMLParseError({ error }),
        }).pipe(Effect.withSpan("XML Parse"));
      },
    };
  },
}) {}
