import { describe, expect, it } from "vitest";
import { IncrementalMarkdownBlockParser } from "./incremental-markdown";
import { parseInlineSegments, parseMarkdownBlocks, type InlineSegment, type MarkdownBlock } from "./parse-markdown";

function stripKeys(blocks: MarkdownBlock[]): unknown[] {
  return blocks.map((block) => {
    const { key: _key, ...rest } = block;
    return rest;
  });
}

describe("IncrementalMarkdownBlockParser", () => {
  it("matches full streaming parse after append-only chunks", () => {
    const parser = new IncrementalMarkdownBlockParser();
    const chunks = [
      "# 标题\n\n",
      "第一段带 [toolcall:call-1]。\n\n",
      "- 项目 A\n",
      "- 项目 B\n\n",
      "```ts\n",
      "console.log('hi')\n",
      "```\n\n",
      "| A | B |\n",
      "|---|---|\n",
      "| 1 | 2 |\n\n",
      "结束。",
    ];

    let text = "";
    let actual: MarkdownBlock[] = [];
    for (const chunk of chunks) {
      text += chunk;
      actual = parser.parse(text);
    }

    expect(stripKeys(actual)).toEqual(stripKeys(parseMarkdownBlocks(text, true)));
  });

  it("resets when the input is replaced instead of appended", () => {
    const parser = new IncrementalMarkdownBlockParser();
    parser.parse("旧内容\n\n- A");

    const text = "新内容\n\n- B";
    expect(stripKeys(parser.parse(text))).toEqual(stripKeys(parseMarkdownBlocks(text, true)));
  });

  it("keeps streaming footnotes at the end", () => {
    const parser = new IncrementalMarkdownBlockParser();
    const chunks = ["脚注引用[^a]", "\n\n继续正文\n\n", "[^a]: 说明"];
    let text = "";
    let actual: MarkdownBlock[] = [];
    for (const chunk of chunks) {
      text += chunk;
      actual = parser.parse(text);
    }

    expect(actual[actual.length - 1]?.type).toBe("footnotes");
    expect(stripKeys(actual)).toEqual(stripKeys(parseMarkdownBlocks(text, true)));
  });

  it("matches full streaming parse for code fences indented up to three spaces", () => {
    const parser = new IncrementalMarkdownBlockParser();
    const chunks = [
      "说明\n\n",
      "  ```ts\n",
      "const value = 1;\n",
      "   ```\n",
      "结束",
    ];
    let text = "";
    let actual: MarkdownBlock[] = [];
    for (const chunk of chunks) {
      text += chunk;
      actual = parser.parse(text);
    }

    expect(stripKeys(actual)).toEqual(stripKeys(parseMarkdownBlocks(text, true)));
    expect(actual.some((block) => block.type === "code" && block.lang === "ts")).toBe(true);
  });
});

describe("parseInlineSegments", () => {
  it("keeps raw delimiters for inline dollar math", () => {
    const segments = parseInlineSegments("能量公式 $E=mc^2$ 很常见");

    expect(segments).toEqual<InlineSegment[]>([
      { type: "text", text: "能量公式 " },
      { type: "math", text: "E=mc^2", raw: "$E=mc^2$", display: false },
      { type: "text", text: " 很常见" },
    ]);
  });

  it("supports LaTeX parenthesized inline math", () => {
    const segments = parseInlineSegments("能量公式 \\(E=mc^2\\) 很常见");

    expect(segments).toEqual<InlineSegment[]>([
      { type: "text", text: "能量公式 " },
      { type: "math", text: "E=mc^2", raw: "\\(E=mc^2\\)", display: false },
      { type: "text", text: " 很常见" },
    ]);
  });

  it("parses simple same-line dollar math without heuristic filtering", () => {
    const segments = parseInlineSegments("求 $x$ 固定，$z$ 跟着 $y$ 变，直接输出$dx,dy$");

    expect(segments).toEqual<InlineSegment[]>([
      { type: "text", text: "求 " },
      { type: "math", text: "x", raw: "$x$", display: false },
      { type: "text", text: " 固定，" },
      { type: "math", text: "z", raw: "$z$", display: false },
      { type: "text", text: " 跟着 " },
      { type: "math", text: "y", raw: "$y$", display: false },
      { type: "text", text: " 变，直接输出" },
      { type: "math", text: "dx,dy", raw: "$dx,dy$", display: false },
    ]);
  });

  it("does not parse inline dollar math across line breaks", () => {
    const segments = parseInlineSegments("跨行 $x\n+y$ 不按行内公式");

    expect(segments).toEqual<InlineSegment[]>([
      { type: "text", text: "跨行 $x\n+y$ 不按行内公式" },
    ]);
  });

  it("does not treat double dollar math as inline math inside paragraphs", () => {
    const segments = parseInlineSegments("不要把 $$E=mc^2$$ 当成行内公式");

    expect(segments).toEqual<InlineSegment[]>([
      { type: "text", text: "不要把 $$E=mc^2$$ 当成行内公式" },
    ]);
  });

  it("renders strong text across inline code spans", () => {
    const segments = parseInlineSegments("**Issue 1: `heading_h1` 一直为空**");

    expect(segments).toEqual<InlineSegment[]>([
      {
        type: "strong",
        children: [
          { type: "text", text: "Issue 1: " },
          { type: "code", text: "heading_h1" },
          { type: "text", text: " 一直为空" },
        ],
      },
    ]);
  });

  it("keeps emphasis markers inside inline code literal", () => {
    const segments = parseInlineSegments("`**not strong**`");

    expect(segments).toEqual<InlineSegment[]>([
      { type: "code", text: "**not strong**" },
    ]);
  });

  it("supports whitelisted inline html tags", () => {
    const segments = parseInlineSegments("按 <kbd>Ctrl</kbd>+<kbd>K</kbd><br>H<sub>2</sub>O 与 x<sup>2</sup>，<mark>重点</mark>");

    expect(segments).toEqual<InlineSegment[]>([
      { type: "text", text: "按 " },
      { type: "html_kbd", children: [{ type: "text", text: "Ctrl" }] },
      { type: "text", text: "+" },
      { type: "html_kbd", children: [{ type: "text", text: "K" }] },
      { type: "html_br" },
      { type: "text", text: "H" },
      { type: "html_sub", children: [{ type: "text", text: "2" }] },
      { type: "text", text: "O 与 x" },
      { type: "html_sup", children: [{ type: "text", text: "2" }] },
      { type: "text", text: "，" },
      { type: "html_mark", children: [{ type: "text", text: "重点" }] },
    ]);
  });
});

describe("parseMarkdownBlocks", () => {
  it("parses code fences indented up to three spaces", () => {
    const blocks = stripKeys(parseMarkdownBlocks([
      "  ```ts",
      "const value = 1;",
      "   ```",
    ].join("\n")));

    expect(blocks).toEqual([
      {
        type: "code",
        lang: "ts",
        text: "const value = 1;",
      },
    ]);
  });

  it("does not parse four-space indented lines as code fences", () => {
    const blocks = stripKeys(parseMarkdownBlocks([
      "    ```ts",
      "const value = 1;",
      "    ```",
    ].join("\n")));

    expect(blocks).toEqual([
      {
        type: "paragraph",
        text: "```ts\nconst value = 1;\n    ```",
      },
    ]);
  });

  it("parses single-line display math blocks with raw delimiters", () => {
    const blocks = stripKeys(parseMarkdownBlocks("$$E=mc^2$$"));

    expect(blocks).toEqual([
      {
        type: "math",
        text: "E=mc^2",
        raw: "$$E=mc^2$$",
      },
    ]);
  });

  it("parses single-line bracketed display math blocks with raw delimiters", () => {
    const blocks = stripKeys(parseMarkdownBlocks("\\[E=mc^2\\]"));

    expect(blocks).toEqual([
      {
        type: "math",
        text: "E=mc^2",
        raw: "\\[E=mc^2\\]",
      },
    ]);
  });

  it("parses display math blocks with content beside double-dollar delimiters", () => {
    const blocks = stripKeys(parseMarkdownBlocks("$$ \\sum_{n=1}^{\\infty} \\frac{1}{n^2} = \\frac{\\pi^2}{6} $$"));

    expect(blocks).toEqual([
      {
        type: "math",
        text: "\\sum_{n=1}^{\\infty} \\frac{1}{n^2} = \\frac{\\pi^2}{6}",
        raw: "$$ \\sum_{n=1}^{\\infty} \\frac{1}{n^2} = \\frac{\\pi^2}{6} $$",
      },
    ]);
  });

  it("parses multiline display math blocks with raw delimiters", () => {
    const blocks = stripKeys(parseMarkdownBlocks([
      "$$",
      "\\sum_{n=1}^{\\infty} \\frac{1}{n^2}",
      "=",
      "\\frac{\\pi^2}{6}",
      "$$",
    ].join("\n")));

    expect(blocks).toEqual([
      {
        type: "math",
        text: "\\sum_{n=1}^{\\infty} \\frac{1}{n^2}\n=\n\\frac{\\pi^2}{6}",
        raw: "$$\n\\sum_{n=1}^{\\infty} \\frac{1}{n^2}\n=\n\\frac{\\pi^2}{6}\n$$",
      },
    ]);
  });

  it("parses multiline bracketed display math blocks with raw delimiters", () => {
    const blocks = stripKeys(parseMarkdownBlocks([
      "\\[",
      "\\sum_{n=1}^{\\infty} \\frac{1}{n^2}",
      "=",
      "\\frac{\\pi^2}{6}",
      "\\]",
    ].join("\n")));

    expect(blocks).toEqual([
      {
        type: "math",
        text: "\\sum_{n=1}^{\\infty} \\frac{1}{n^2}\n=\n\\frac{\\pi^2}{6}",
        raw: "\\[\n\\sum_{n=1}^{\\infty} \\frac{1}{n^2}\n=\n\\frac{\\pi^2}{6}\n\\]",
      },
    ]);
  });

  it("preserves quoted display math blocks for nested rendering", () => {
    const blocks = stripKeys(parseMarkdownBlocks([
      "> The force of interest, \\( \\delta(t) \\), is a function of time ...",
      ">",
      "> \\[",
      "> \\delta(t)=",
      "> \\begin{cases}",
      "> 0.07-0.005t, & t\\le 8,\\\\",
      "> 0.06, & t>8.",
      "> \\end{cases}",
      "> \\]",
      ">",
      "> Calculate the present value ...",
    ].join("\n")));

    expect(blocks).toEqual([
      {
        type: "quote",
        text: [
          "The force of interest, \\( \\delta(t) \\), is a function of time ...",
          "",
          "\\[",
          "\\delta(t)=",
          "\\begin{cases}",
          "0.07-0.005t, & t\\le 8,\\\\",
          "0.06, & t>8.",
          "\\end{cases}",
          "\\]",
          "",
          "Calculate the present value ...",
        ].join("\n"),
      },
    ]);
  });

  it("preserves quoted double-dollar display math blocks for nested rendering", () => {
    const blocks = stripKeys(parseMarkdownBlocks([
      "> $$",
      "> E = mc^2",
      "> $$",
    ].join("\n")));

    expect(blocks).toEqual([
      {
        type: "quote",
        text: "$$\nE = mc^2\n$$",
      },
    ]);
  });

  it("supports whitelisted details blocks", () => {
    const blocks = stripKeys(parseMarkdownBlocks([
      "<details open>",
      "<summary>展开看 <mark>说明</mark></summary>",
      "",
      "正文第一行",
      "",
      "- 项目 A",
      "</details>",
    ].join("\n")));

    expect(blocks).toEqual([
      {
        type: "details",
        summary: "展开看 <mark>说明</mark>",
        body: "正文第一行\n\n- 项目 A",
        open: true,
      },
    ]);
  });

  it("preserves ordered-list numbering after an intervening unordered list", () => {
    const blocks = stripKeys(parseMarkdownBlocks([
      "1. 找出全部读取点",
      "2. 区分：",
      "   - 真正在产品链路里用的",
      "   - 只在测试/调试里用的",
      "3. 逐类迁到 metadata / snapshot / overview 真相层",
    ].join("\n")));

    expect(blocks).toEqual([
      {
        type: "list",
        ordered: true,
        items: [
          { text: "找出全部读取点", marker: "1.", value: 1 },
          { text: "区分：", marker: "2.", value: 2 },
        ],
      },
      {
        type: "list",
        ordered: false,
        items: [
          { text: "真正在产品链路里用的", marker: "-", value: undefined },
          { text: "只在测试/调试里用的", marker: "-", value: undefined },
        ],
      },
      {
        type: "list",
        ordered: true,
        items: [
          { text: "逐类迁到 metadata / snapshot / overview 真相层", marker: "3.", value: 3 },
        ],
      },
    ]);
  });

  it("preserves per-item ordered values from markdown source", () => {
    const blocks = stripKeys(parseMarkdownBlocks([
      "1. 第一项",
      "135. 第一百三十五项",
    ].join("\n")));

    expect(blocks).toEqual([
      {
        type: "list",
        ordered: true,
        items: [
          { text: "第一项", marker: "1.", value: 1 },
          { text: "第一百三十五项", marker: "135.", value: 135 },
        ],
      },
    ]);
  });

  it("keeps table cells with escaped pipes (math norm) intact", () => {
    const blocks = stripKeys(parseMarkdownBlocks([
      "| 性质 | 意义 | 直觉 |",
      "| :--- | :--- | :--- |",
      "| **非负性** | $\\|x\\| \\ge 0$ | 长度不能为负 |",
      "| **正定性** | $\\|x\\| = 0 \\Leftrightarrow x = 0$ | 只有零向量 |",
    ].join("\n")));

    expect(blocks).toEqual([
      {
        type: "table",
        headers: ["性质", "意义", "直觉"],
        rows: [
          ["**非负性**", "$\\|x\\| \\ge 0$", "长度不能为负"],
          ["**正定性**", "$\\|x\\| = 0 \\Leftrightarrow x = 0$", "只有零向量"],
        ],
      },
    ]);
  });
});
