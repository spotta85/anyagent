package anyagent

import (
	"encoding/json"
	"testing"
)

// A variant from a newer binary decodes as Unrecognized instead of failing the frame.
func TestUnrecognizedVariantsDecode(t *testing.T) {
	for _, raw := range []string{`{"NewThing": {}}`, `"NewThing"`} {
		var kind EventKind
		if err := json.Unmarshal([]byte(raw), &kind); err != nil || kind.Name() != "NewThing" {
			t.Fatal(raw, err, kind.Name())
		}
	}
	var tool ToolUpdate
	ok(t, json.Unmarshal([]byte(`{"id": "t", "kind": "Web", "title": "", "status": "Paused", "input": "None", "diffs": [], "locations": []}`), &tool))
	if tool.Kind.Unrecognized != "Web" || tool.Status != "Paused" || !tool.Input.None {
		t.Fatal(tool.Kind, tool.Status, tool.Input)
	}
}
