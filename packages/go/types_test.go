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

// An agent reference round-trips as the variant its keys name, never as a zero AgentAt.
func TestAgentRefVariantsRoundTrip(t *testing.T) {
	for raw, name := range map[string]string{
		`"claude"`:                             "string",
		`{"id":"claude","path":"/opt/claude"}`: "AgentAt",
		`{"acp":{"name":"x","path":"/p"}}`:     "acp",
		`{"NewThing":{}}`:                      "NewThing",
	} {
		var ref AgentRef
		if err := json.Unmarshal([]byte(raw), &ref); err != nil || ref.Name() != name {
			t.Fatal(raw, err, ref.Name())
		}
	}
}
