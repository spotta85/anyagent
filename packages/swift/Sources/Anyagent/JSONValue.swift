// Untyped JSON for `extensions`, raw tool input, error extras, and command
// frames, plus the Codable helpers the generated enums use.

import Foundation

/// Any JSON value.
public enum JSONValue: Codable, Sendable, Equatable {
    case null
    case bool(Bool)
    case number(Double)
    case string(String)
    case array([JSONValue])
    case object([String: JSONValue])

    public init(from decoder: Decoder) throws {
        let c = try decoder.singleValueContainer()
        if c.decodeNil() {
            self = .null
        } else if let v = try? c.decode(Bool.self) {
            self = .bool(v)
        } else if let v = try? c.decode(Double.self) {
            self = .number(v)
        } else if let v = try? c.decode(String.self) {
            self = .string(v)
        } else if let v = try? c.decode([JSONValue].self) {
            self = .array(v)
        } else {
            self = .object(try c.decode([String: JSONValue].self))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var c = encoder.singleValueContainer()
        switch self {
        case .null: try c.encodeNil()
        case .bool(let v): try c.encode(v)
        case .number(let v): try c.encode(v)
        case .string(let v): try c.encode(v)
        case .array(let v): try c.encode(v)
        case .object(let v): try c.encode(v)
        }
    }
}

extension JSONValue: ExpressibleByStringLiteral, ExpressibleByBooleanLiteral, ExpressibleByIntegerLiteral,
    ExpressibleByFloatLiteral, ExpressibleByArrayLiteral, ExpressibleByDictionaryLiteral, ExpressibleByNilLiteral
{
    public init(stringLiteral v: String) { self = .string(v) }
    public init(booleanLiteral v: Bool) { self = .bool(v) }
    public init(integerLiteral v: Int) { self = .number(Double(v)) }
    public init(floatLiteral v: Double) { self = .number(v) }
    public init(arrayLiteral v: JSONValue...) { self = .array(v) }
    public init(dictionaryLiteral v: (String, JSONValue)...) { self = .object(Dictionary(uniqueKeysWithValues: v)) }
    public init(nilLiteral: ()) { self = .null }
}

// ---------------------------------------------------------------------------
// CODABLE HELPERS for the generated enums
// ---------------------------------------------------------------------------

/// A coding key that is just its string.
struct Key: CodingKey {
    let stringValue: String
    var intValue: Int? { nil }
    init(_ s: String) { stringValue = s }
    init?(stringValue: String) { self.stringValue = stringValue }
    init?(intValue: Int) { nil }
}

extension Encoder {
    /// `{"Tag": value}`: a variant with a payload.
    func tagged<T: Encodable>(_ tag: String, _ value: T) throws {
        var c = container(keyedBy: Key.self)
        try c.encode(value, forKey: Key(tag))
    }

    /// The value itself: a unit variant's name, or an untagged member.
    func raw<T: Encodable>(_ value: T) throws {
        var c = singleValueContainer()
        try c.encode(value)
    }
}

func unknownVariant(_ decoder: Decoder, _ got: String) -> DecodingError {
    .dataCorrupted(.init(codingPath: decoder.codingPath, debugDescription: "unknown variant \(got)"))
}
