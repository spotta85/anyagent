#!/usr/bin/env python3
"""packages/schema.json -> packages/swift/Sources/Anyagent/Types.swift and
packages/go/types.go. `just types` runs it; CI checks the output is current.

Every definition becomes a struct, a string enum, or an enum with payloads
(Rust's externally tagged enums: `{"TextDelta": {..}}` or a bare string
`"ContextCompacted"`); a variant the schema does not list decodes as
`unrecognized` with its wire name, so a newer binary never fails a frame. Frames, lines, the hello and the error body are
hand-written in each wrapper, so they are skipped here."""

from __future__ import annotations

import json
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path

HERE = Path(__file__).resolve().parent
SKIP = {"Frame", "Line", "Hello", "ErrorBody"}
HEADER = "Generated from packages/schema.json by `just types`. Do not edit."

# A type is a tuple: ("str",) ("bool",) ("int", fmt) ("num",) ("any",)
# ("ref", name) ("array", t) ("map", t) ("opt", t).
Type = tuple


@dataclass
class Struct:
    name: str
    doc: str
    fields: list[tuple[str, Type, bool]]  # wire name, type, required


@dataclass
class StringEnum:
    name: str
    doc: str
    values: list[str]


@dataclass
class Enum:
    name: str
    doc: str
    variants: list[tuple[str, str | None, Type | None]]  # ("unit", name, None) | ("tagged", name, t) | ("raw", None, t)


# ---------------------------------------------------------------------------
# MAIN
# ---------------------------------------------------------------------------


def main() -> None:
    """Parses every definition once, then prints each language's version."""
    defs = json.load(open(HERE / "schema.json"))["definitions"]
    types = Parser(defs).run()
    (HERE / "swift/Sources/Anyagent/Types.swift").write_text(swift(types))
    go_file = HERE / "go/types.go"
    go_file.write_text(go(types))
    if shutil.which("gofmt"):
        subprocess.run(["gofmt", "-w", go_file], check=True)


# ---------------------------------------------------------------------------
# PARSER: schema -> Struct | StringEnum | Enum
# ---------------------------------------------------------------------------


class Parser:
    """Walks the definitions; an inline object becomes a struct named after where it sits."""

    def __init__(self, defs: dict) -> None:
        self.defs = defs
        self.types: list = []
        self.taken = set(defs)

    def run(self) -> list:
        for name, schema in self.defs.items():
            if name not in SKIP:
                self.define(name, schema)
        return self.types

    def define(self, name: str, s: dict) -> None:
        """Adds one named type."""
        self.taken.add(name)
        doc = (s.get("description") or "").split("\n")[0]
        if "enum" in s:
            self.types.append(StringEnum(name, doc, s["enum"]))
        elif "oneOf" in s or "anyOf" in s:
            variants = self.variants(name, s.get("oneOf") or s["anyOf"])
            if all(kind == "unit" for kind, _, _ in variants):
                self.types.append(StringEnum(name, doc, [n for _, n, _ in variants]))
            else:
                self.types.append(Enum(name, doc, variants))
        else:
            assert s.get("type") == "object", name
            required = set(s.get("required", []))
            fields = [(k, self.typeof(v, name + pascal(k)), k in required) for k, v in s.get("properties", {}).items()]
            self.types.append(Struct(name, doc, fields))

    def variants(self, enum: str, members: list) -> list:
        """A unit is a string const; a tagged variant is an object with one required key; anything else is raw."""
        out = []
        for m in members:
            if m.get("type") == "string" and "const" in m:
                out.append(("unit", m["const"], None))
            elif m.get("type") == "string" and "enum" in m:
                out += [("unit", u, None) for u in m["enum"]]
            elif m.get("type") == "object" and m.get("required") == list(m.get("properties", {})) and len(m["required"]) == 1:
                ((tag, payload),) = m["properties"].items()
                out.append(("tagged", tag, self.typeof(payload, tag if tag not in self.taken else enum + tag)))
            else:
                out.append(("raw", None, self.typeof(m, enum)))
        return out

    def typeof(self, s: dict | bool, inline: str) -> Type:
        """The type a property schema names; an inline object is defined as `inline`."""
        if s is True:
            return ("any",)
        if "$ref" in s:
            return ("ref", s["$ref"].rsplit("/", 1)[1])
        if "allOf" in s:
            return self.typeof(s["allOf"][0], inline)
        if "anyOf" in s:
            (member,) = [m for m in s["anyOf"] if m.get("type") != "null"]
            return ("opt", self.typeof(member, inline))
        t = s.get("type")
        if isinstance(t, list):
            (t,) = [x for x in t if x != "null"]
            return ("opt", self.typeof({**s, "type": t}, inline))
        if t == "object" and "properties" in s:
            self.define(inline, s)
            return ("ref", inline)
        if t == "object":
            extra = s.get("additionalProperties", True)
            return ("map", ("any",) if extra is True else self.typeof(extra, inline))
        if t == "array":
            return ("array", self.typeof(s["items"], inline))
        if t == "integer":
            return ("int", s.get("format", ""))
        return {"string": ("str",), "boolean": ("bool",), "number": ("num",), None: ("any",)}[t]


# ---------------------------------------------------------------------------
# SWIFT
# ---------------------------------------------------------------------------

SWIFT_KEYWORDS = set(
    "as break case catch class continue default defer do else enum extension false fallthrough for func "
    "guard if import in init inout internal is let nil none operator private protocol public repeat rethrows "
    "return self static struct subscript super switch throw throws true try typealias var where while Self Type".split()
)


def swift(types: list) -> str:
    """The Swift file: structs, `String` enums, and enums with payloads that decode both wire forms."""
    parts = [f"// {HEADER}\n\nimport Foundation"]
    for t in types:
        if isinstance(t, Struct):
            parts.append(swift_struct(t))
        elif isinstance(t, StringEnum):
            parts.append(swift_string_enum(t))
        else:
            parts.append(swift_enum(t))
    return "\n\n".join(parts) + "\n"


def swift_struct(s: Struct) -> str:
    fields = [(swift_name(j), swift_type(t if req else opt(t)), j) for j, t, req in s.fields]
    lines = swift_doc(s.doc) + [f"public struct {s.name}: Codable, Sendable, Equatable {{"]
    lines += [f"    public var {n}: {ty}" for n, ty, _ in fields]
    params = ", ".join(f"{n}: {ty}" + (" = nil" if ty.endswith("?") else "") for n, ty, _ in fields)
    lines += ["", f"    public init({params}) {{"] + [f"        self.{n} = {n}" for n, _, _ in fields] + ["    }"]
    if any(n != j for n, _, j in fields):
        lines += ["", "    enum CodingKeys: String, CodingKey {"]
        lines += [f"        case {n}" + (f' = "{j}"' if n != j else "") for n, _, j in fields] + ["    }"]
    return "\n".join(lines + ["}"])


def swift_string_enum(e: StringEnum) -> str:
    lines = swift_doc(e.doc) + [f"public enum {e.name}: String, Codable, Sendable, Equatable {{"]
    lines += [f'    case {swift_name(v)} = "{v}"' for v in e.values]
    lines += ["    /// A value this package does not know (a newer binary).", "    case unrecognized", ""]
    lines += ["    public init(from decoder: Decoder) throws {", "        self = Self(rawValue: try String(from: decoder)) ?? .unrecognized", "    }"]
    return "\n".join(lines + ["}"])


def swift_enum(e: Enum) -> str:
    cases = [(kind, swift_name(n) if n else raw_name(t, swift_name), n or raw_name(t, str), t) for kind, n, t in e.variants]
    units = [(c, n) for kind, c, n, _ in cases if kind == "unit"]
    raws = [(c, t) for kind, c, _, t in cases if kind == "raw"]
    tagged = [(c, n, t) for kind, c, n, t in cases if kind == "tagged"]
    raw_string = next((c for c, t in raws if t == ("str",)), None)
    fallback = raw_string or "unrecognized"  # a string no unit matched

    lines = swift_doc(e.doc) + [f"public enum {e.name}: Codable, Sendable, Equatable {{"]
    lines += [f"    case {c}" + (f"({swift_type(t)})" if t else "") for _, c, _, t in cases]
    lines += ["    /// A variant this package does not know (a newer binary): its wire name.", "    case unrecognized(String)"]
    lines += ["", f'    /// The variant\'s wire name: "{cases[0][2]}", …', "    public var name: String {", "        switch self {"]
    lines += [f'        case .{c}: "{n}"' for _, c, n, _ in cases] + ["        case .unrecognized(let tag): tag", "        }", "    }"]

    lines += ["", "    public init(from decoder: Decoder) throws {"]
    if units:
        lines += ["        if let s = try? String(from: decoder) {", "            switch s {"]
        lines += [f'            case "{n}": self = .{c}' for c, n in units]
        lines += [f"            default: self = .{fallback}(s)", "            }", "            return", "        }"]
    else:
        lines.append(f"        if let s = try? String(from: decoder) {{ self = .{fallback}(s); return }}")
    for c, t in raws:
        if t != ("str",):
            lines.append(f"        if let v = try? {swift_type(t)}(from: decoder) {{ self = .{c}(v); return }}")
    if tagged:
        lines += ["        let c = try decoder.container(keyedBy: Key.self)", "        switch c.allKeys.first?.stringValue {"]
        lines += [f'        case "{n}": self = .{c}(try c.decode({swift_type(t)}.self, forKey: Key("{n}")))' for c, n, t in tagged]
        lines += ['        default: self = .unrecognized(c.allKeys.first?.stringValue ?? "?")', "        }"]
    else:
        lines.append('        self = .unrecognized("?")')
    lines.append("    }")

    lines += ["", "    public func encode(to encoder: Encoder) throws {", "        switch self {"]
    for kind, c, n, _ in cases:
        if kind == "unit":
            lines.append(f'        case .{c}: try encoder.raw("{n}")')
        elif kind == "raw":
            lines.append(f"        case .{c}(let v): try encoder.raw(v)")
        else:
            lines.append(f'        case .{c}(let v): try encoder.tagged("{n}", v)')
    lines += ["        case .unrecognized(let tag): try encoder.raw(tag)", "        }", "    }", "}"]

    if raw_string:
        lines += ["", f"extension {e.name}: ExpressibleByStringLiteral {{", f"    public init(stringLiteral v: String) {{ self = .{raw_string}(v) }}", "}"]
    if any(t == ("bool",) for _, t in raws):
        lines += ["", f"extension {e.name}: ExpressibleByBooleanLiteral {{", "    public init(booleanLiteral v: Bool) { self = .bool(v) }", "}"]
    return "\n".join(lines)


def swift_type(t: Type) -> str:
    match t:
        case ("str",):
            return "String"
        case ("bool",):
            return "Bool"
        case ("int", "uint64"):
            return "UInt64"
        case ("int", "uint32"):
            return "UInt32"
        case ("int", "uint8"):
            return "UInt8"
        case ("int", _):
            return "Int"
        case ("num",):
            return "Double"
        case ("any",):
            return "JSONValue"
        case ("ref", name):
            return name
        case ("array", inner):
            return f"[{swift_type(inner)}]"
        case ("map", inner):
            return f"[String: {swift_type(inner)}]"
        case ("opt", inner):
            return f"{swift_type(inner)}?"
    raise ValueError(t)


def swift_name(wire: str) -> str:
    """`message_id` -> `messageId`, `AllowOnce` -> `allowOnce`, keywords in backticks."""
    name = camel(wire)
    return f"`{name}`" if name in SWIFT_KEYWORDS else name


def swift_doc(doc: str) -> list[str]:
    return [f"/// {doc}"] if doc else []


# ---------------------------------------------------------------------------
# GO
# ---------------------------------------------------------------------------

GO_ACRONYMS = {"id": "ID", "url": "URL", "usd": "USD"}


def go(types: list) -> str:
    """The Go file: structs, string types with consts, and one-field-set structs for enums with payloads."""
    parts = [f"// Code generated from packages/schema.json by `just types`. DO NOT EDIT.\n\npackage anyagent\n\nimport (\n\t\"encoding/json\"\n\t\"fmt\"\n)"]
    for t in types:
        if isinstance(t, Struct):
            parts.append(go_struct(t))
        elif isinstance(t, StringEnum):
            parts.append(go_string_enum(t))
        else:
            parts.append(go_enum(t))
    return "\n\n".join(parts) + "\n"


def go_struct(s: Struct) -> str:
    lines = [go_doc(s.name, s.doc), f"type {s.name} struct {{"]
    for j, t, req in s.fields:
        lines.append(f'\t{go_field(j)} {go_type(t if req else opt(t))} `json:"{j}{"" if req else ",omitempty"}"`')
    return "\n".join(lines + ["}"])


def go_string_enum(e: StringEnum) -> str:
    lines = [go_doc(e.name, e.doc), f"type {e.name} string", "", "const ("]
    lines += [f'\t{e.name}{v} {e.name} = "{v}"' for v in e.values]
    return "\n".join(lines + [")"])


def go_enum(e: Enum) -> str:
    fields = [(kind, pascal(n) if n else raw_name(t, pascal), n or raw_name(t, str), t) for kind, n, t in e.variants]
    units = [(f, n) for kind, f, n, _ in fields if kind == "unit"]
    raws = [(f, t) for kind, f, _, t in fields if kind == "raw"]
    tagged = [f for kind, f, _, _ in fields if kind == "tagged"]
    raw_string = next((f for f, t in raws if t == ("str",)), None)

    lines = [go_doc(e.name, f"exactly one field is set. {e.doc}".strip()), f"type {e.name} struct {{"]
    for kind, f, n, t in fields:
        if kind == "unit":
            lines.append(f'\t{f} bool `json:"-"`')
        elif kind == "tagged":
            lines.append(f'\t{f} {go_type(opt(t))} `json:"{n}"`')
        else:
            lines.append(f'\t{f} {go_type(opt(t))} `json:"-"`')
    lines += ['\tUnrecognized string `json:"-"` // a variant this package does not know (a newer binary): its wire name', "}"]

    lines += ["", f'// Name is the variant\'s wire name: "{fields[0][2]}", …', f"func (v {e.name}) Name() string {{", "\tswitch {"]
    lines += [f"\tcase {go_set(kind, f)}:\n\t\treturn \"{n}\"" for kind, f, n, _ in fields]
    lines += ["\t}", "\treturn v.Unrecognized", "}"]

    lines += ["", f"func (v {e.name}) MarshalJSON() ([]byte, error) {{", "\tswitch {"]
    for kind, f, n, _ in fields:
        value = f'"{n}"' if kind == "unit" else f"v.{f}" if kind == "raw" else f'map[string]any{{"{n}": v.{f}}}'
        lines.append(f"\tcase {go_set(kind, f)}:\n\t\treturn json.Marshal({value})")
    lines += ['\tcase v.Unrecognized != "":\n\t\treturn json.Marshal(v.Unrecognized)']
    lines += ["\t}", f'\treturn nil, fmt.Errorf("{e.name}: no variant set")', "}"]

    lines += ["", f"func (v *{e.name}) UnmarshalJSON(b []byte) error {{", "\tvar s string", "\tif json.Unmarshal(b, &s) == nil {"]
    if units:
        lines += ["\t\tswitch s {"] + [f'\t\tcase "{n}":\n\t\t\tv.{f} = true\n\t\t\treturn nil' for f, n in units] + ["\t\t}"]
    lines += [f"\t\tv.{raw_string} = &s" if raw_string else "\t\tv.Unrecognized = s", "\t\treturn nil", "\t}"]
    for f, t in raws:
        if t != ("str",):
            lines += [f"\tvar raw {go_type(t)}", "\tif json.Unmarshal(b, &raw) == nil {", f"\t\tv.{f} = &raw", "\t\treturn nil", "\t}"]
    if tagged:  # a tag nobody matched: the object's one key
        lines += [f"\ttype plain {e.name}", '\tif err := json.Unmarshal(b, (*plain)(v)); err != nil || v.Name() != "" {', "\t\treturn err", "\t}"]
        lines += ["\tvar m map[string]json.RawMessage", "\tjson.Unmarshal(b, &m)", "\tfor tag := range m {", "\t\tv.Unrecognized = tag", "\t}"]
    else:
        lines.append("\tv.Unrecognized = string(b)")
    lines += ["\treturn nil", "}"]
    return "\n".join(lines)


def go_set(kind: str, field: str) -> str:
    return f"v.{field}" if kind == "unit" else f"v.{field} != nil"


def go_type(t: Type) -> str:
    match t:
        case ("str",):
            return "string"
        case ("bool",):
            return "bool"
        case ("int", "uint64"):
            return "uint64"
        case ("int", "uint32"):
            return "uint32"
        case ("int", "uint8"):
            return "uint8"
        case ("int", _):
            return "int64"
        case ("num",):
            return "float64"
        case ("any",):
            return "any"
        case ("ref", name):
            return name
        case ("array", inner):
            return f"[]{go_type(inner)}"
        case ("map", inner):
            return f"map[string]{go_type(inner)}"
        case ("opt", inner):  # slices, maps and any are already nil-able
            return go_type(inner) if inner[0] in ("array", "map", "any") else f"*{go_type(inner)}"
    raise ValueError(t)


def go_field(wire: str) -> str:
    """`message_id` -> `MessageID`."""
    return "".join(GO_ACRONYMS.get(p, p.capitalize()) for p in wire.split("_"))


def go_doc(name: str, doc: str) -> str:
    return f"// {name}: {doc}" if doc else f"// {name} is a wire type."


# ---------------------------------------------------------------------------
# HELPERS
# ---------------------------------------------------------------------------


def opt(t: Type) -> Type:
    return t if t[0] == "opt" else ("opt", t)


def pascal(wire: str) -> str:
    return "".join(p[:1].upper() + p[1:] for p in wire.split("_"))


def camel(wire: str) -> str:
    p = pascal(wire)
    return p[:1].lower() + p[1:]


def raw_name(t: Type, style) -> str:
    """What an untagged member is called: `string`, `bool`, or the referenced type."""
    base = {"str": "string", "bool": "bool", "int": "int", "num": "number", "any": "json"}.get(t[0]) or (t[1] if t[0] == "ref" else t[0])
    return style(base)


if __name__ == "__main__":
    main()
