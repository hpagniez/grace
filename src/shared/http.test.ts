import { describe, expect, it } from "vitest";
import { HTTP_GROUPS, curlCommand, httpGroup, prettyBody, type HttpCall } from "./http";

const call = (over: Partial<HttpCall> = {}): HttpCall => ({
  method: "GET",
  url: "http://h/x",
  requestHeaders: [],
  requestBody: null,
  status: 200,
  responseHeaders: [],
  responseBody: null,
  millis: 12,
  error: null,
  via: null,
  startTime: 1792937525123,
  attempt: null,
  ...over,
});

describe("httpGroup", () => {
  it("classifies by hundred", () => {
    expect(httpGroup(100)).toBe("1xx");
    expect(httpGroup(199)).toBe("1xx");
    expect(httpGroup(200)).toBe("2xx");
    expect(httpGroup(302)).toBe("3xx");
    expect(httpGroup(404)).toBe("4xx");
    expect(httpGroup(500)).toBe("5xx");
    expect(httpGroup(599)).toBe("5xx");
    expect(httpGroup(null)).toBe("none");
    expect(httpGroup(99)).toBeNull();
    expect(httpGroup(600)).toBeNull();
  });
});

describe("HTTP_GROUPS", () => {
  it("lists the six groups in order", () => {
    expect(HTTP_GROUPS.map((g) => g.group)).toEqual(["1xx", "2xx", "3xx", "4xx", "5xx", "none"]);
    expect(HTTP_GROUPS.map((g) => g.label)).toEqual([
      "1xx Informational",
      "2xx Success",
      "3xx Redirect",
      "4xx Client error",
      "5xx Server error",
      "No response",
    ]);
  });
});

describe("curlCommand", () => {
  it("drops the headers curl sets", () => {
    const c = call({
      url: "http://h/x?q=a%20b",
      requestHeaders: [
        { name: "Host", value: "h" },
        { name: "content-length", value: "0" },
        { name: "Connection", value: "keep-alive" },
        { name: "Accept", value: "application/json" },
        { name: "Authorization", value: "***" },
      ],
    });
    expect(curlCommand(c)).toBe(
      "curl -X GET 'http://h/x?q=a%20b' -H 'Accept: application/json' -H 'Authorization: ***'",
    );
  });

  it("adds the body as logged", () => {
    const c = call({ method: "POST", requestBody: '{\n  "a": "it\'s"\n}' });
    expect(curlCommand(c)).toBe(
      "curl -X POST 'http://h/x' --data-raw '{\n  \"a\": \"it'\\''s\"\n}'",
    );
  });

  it("quotes every value", () => {
    const c = call({
      url: "http://h/it's",
      requestHeaders: [{ name: "X-A", value: "$HOME `id`" }],
    });
    expect(curlCommand(c)).toBe("curl -X GET 'http://h/it'\\''s' -H 'X-A: $HOME `id`'");
  });

  it("quotes a method that is not plain letters", () => {
    expect(curlCommand(call({ method: "$(id)" }))).toBe("curl -X '$(id)' 'http://h/x'");
  });

  it("is null without a URL", () => {
    expect(curlCommand(call({ url: null }))).toBeNull();
  });
});

describe("prettyBody", () => {
  it("re-indents JSON", () => {
    const x = '{"a":1,"b":[1,2]}';
    expect(prettyBody(x)).toBe(JSON.stringify(JSON.parse(x), null, 2));
    const indented = JSON.stringify({ a: 1, b: [1, 2] }, null, 2);
    expect(prettyBody(indented)).toBe(indented);
  });

  it("keeps number text", () => {
    expect(prettyBody('{"id":12345678901234567890,"price":1.50}')).toBe(
      '{\n  "id": 12345678901234567890,\n  "price": 1.50\n}',
    );
  });

  it("keeps punctuation inside strings", () => {
    expect(prettyBody('{"a":"x,{y}:[z]\\"q"}')).toBe('{\n  "a": "x,{y}:[z]\\"q"\n}');
    expect(prettyBody('{"a":"p\\\\","b":"q"}')).toBe('{\n  "a": "p\\\\",\n  "b": "q"\n}');
  });

  it("keeps empty containers on one line", () => {
    expect(prettyBody('{"a":{},"b":[]}')).toBe('{\n  "a": {},\n  "b": []\n}');
    expect(prettyBody("[ { } , [ ] ]")).toBe("[\n  {},\n  []\n]");
  });

  it("matches JSON.stringify for nested ordinary input", () => {
    const x = ' {"a":[{"b":null,"c":[1,[2,{"d":true}]]},"s"],\n"e":{"f":-1.5}} ';
    expect(prettyBody(x)).toBe(JSON.stringify(JSON.parse(x), null, 2));
  });

  it("leaves a JSON scalar as logged", () => {
    expect(prettyBody("1.0")).toBe("1.0");
    expect(prettyBody("null")).toBe("null");
  });

  it("indents XML", () => {
    expect(prettyBody("<user><name>bob</name></user>")).toBe("<user>\n  <name>bob</name>\n</user>");
    expect(prettyBody('<?xml version="1.0"?><a x="1"><b/><c>t</c></a>')).toBe(
      '<?xml version="1.0"?>\n<a x="1">\n  <b/>\n  <c>t</c>\n</a>',
    );
  });

  it("indents XML that is already indented, and keeps comments and empty elements", () => {
    const pretty = "<a>\n  <!-- 1 > 0 -->\n  <b></b>\n  <c>t</c>\n</a>";
    expect(prettyBody(pretty)).toBe(pretty);
  });

  it("leaves anything else as logged", () => {
    for (const x of [
      "{not json",
      "<a><b></a>",
      '<a x="1>2">t</a>',
      "<a><![CDATA[<b>]]></a>",
      "<a>1 < 2</a>",
      "<a></a> trailing",
      "<a>",
      "plain text",
      "",
    ])
      expect(prettyBody(x)).toBe(x);
  });

  it("handles a 200 KB JSON array", () => {
    const rows = Array.from({ length: 3000 }, (_, i) => ({
      id: i,
      name: `user-${i}`,
      email: `user-${i}@example.com`,
      tags: ["a", "b"],
    }));
    const body = JSON.stringify(rows);
    expect(body.length).toBeGreaterThan(200_000);
    const out = prettyBody(body);
    expect(out).toBe(JSON.stringify(rows, null, 2));
    expect(JSON.parse(out)).toEqual(rows);
  });
});
