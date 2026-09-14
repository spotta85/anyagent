import Foundation
import Testing

@testable import Anyagent

/// A variant from a newer binary decodes as `.unrecognized` instead of failing the frame.
@Test func unrecognized_variants_decode() throws {
    func decode<T: Decodable>(_ json: String) throws -> T { try JSONDecoder().decode(T.self, from: Data(json.utf8)) }
    #expect(try decode(#"{"NewThing": {}}"#) == EventKind.unrecognized("NewThing"))
    #expect(try decode(#""NewThing""#) == EventKind.unrecognized("NewThing"))
    let tool: ToolUpdate = try decode(#"{"id": "t", "kind": "Web", "title": "", "status": "Paused", "input": "None", "diffs": [], "locations": []}"#)
    #expect(tool.kind == .unrecognized("Web"))
    #expect(tool.status == .unrecognized)
    #expect(tool.input == .none)
}
