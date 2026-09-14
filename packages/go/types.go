// Code generated from packages/schema.json by `just types`. DO NOT EDIT.

package anyagent

import (
	"encoding/json"
	"fmt"
)

// AgentRef: exactly one field is set. A catalog id like `"claude"`, or an ACP agent the catalog does not know.
type AgentRef struct {
	String       *string  `json:"-"`
	Acp          *AcpSpec `json:"acp"`
	Unrecognized string   `json:"-"` // a variant this package does not know (a newer binary): its wire name
}

// Name is the variant's wire name: "string", …
func (v AgentRef) Name() string {
	switch {
	case v.String != nil:
		return "string"
	case v.Acp != nil:
		return "acp"
	}
	return v.Unrecognized
}

func (v AgentRef) MarshalJSON() ([]byte, error) {
	switch {
	case v.String != nil:
		return json.Marshal(v.String)
	case v.Acp != nil:
		return json.Marshal(map[string]any{"acp": v.Acp})
	case v.Unrecognized != "":
		return json.Marshal(v.Unrecognized)
	}
	return nil, fmt.Errorf("AgentRef: no variant set")
}

func (v *AgentRef) UnmarshalJSON(b []byte) error {
	var s string
	if json.Unmarshal(b, &s) == nil {
		v.String = &s
		return nil
	}
	type plain AgentRef
	if err := json.Unmarshal(b, (*plain)(v)); err != nil || v.Name() != "" {
		return err
	}
	var m map[string]json.RawMessage
	json.Unmarshal(b, &m)
	for tag := range m {
		v.Unrecognized = tag
	}
	return nil
}

// AcpSpec is a wire type.
type AcpSpec struct {
	Name string   `json:"name"`
	Path string   `json:"path"`
	Args []string `json:"args,omitempty"`
}

// PermissionMode: How anyagent handles tool permission requests.
type PermissionMode string

const (
	PermissionModeAsk         PermissionMode = "Ask"
	PermissionModeAutoApprove PermissionMode = "AutoApprove"
)

// McpServer: A client-owned MCP server the agent should connect to, forwarded at open.
type McpServer struct {
	Name       string        `json:"name"`
	Connection McpConnection `json:"connection"`
}

// Stdio is a wire type.
type Stdio struct {
	Command string            `json:"command"`
	Args    []string          `json:"args"`
	Env     map[string]string `json:"env"`
}

// Http is a wire type.
type Http struct {
	URL     string            `json:"url"`
	Headers map[string]string `json:"headers"`
}

// Sse is a wire type.
type Sse struct {
	URL     string            `json:"url"`
	Headers map[string]string `json:"headers"`
}

// McpConnection: exactly one field is set.
type McpConnection struct {
	Stdio        *Stdio `json:"Stdio"`
	Http         *Http  `json:"Http"`
	Sse          *Sse   `json:"Sse"`
	Unrecognized string `json:"-"` // a variant this package does not know (a newer binary): its wire name
}

// Name is the variant's wire name: "Stdio", …
func (v McpConnection) Name() string {
	switch {
	case v.Stdio != nil:
		return "Stdio"
	case v.Http != nil:
		return "Http"
	case v.Sse != nil:
		return "Sse"
	}
	return v.Unrecognized
}

func (v McpConnection) MarshalJSON() ([]byte, error) {
	switch {
	case v.Stdio != nil:
		return json.Marshal(map[string]any{"Stdio": v.Stdio})
	case v.Http != nil:
		return json.Marshal(map[string]any{"Http": v.Http})
	case v.Sse != nil:
		return json.Marshal(map[string]any{"Sse": v.Sse})
	case v.Unrecognized != "":
		return json.Marshal(v.Unrecognized)
	}
	return nil, fmt.Errorf("McpConnection: no variant set")
}

func (v *McpConnection) UnmarshalJSON(b []byte) error {
	var s string
	if json.Unmarshal(b, &s) == nil {
		v.Unrecognized = s
		return nil
	}
	type plain McpConnection
	if err := json.Unmarshal(b, (*plain)(v)); err != nil || v.Name() != "" {
		return err
	}
	var m map[string]json.RawMessage
	json.Unmarshal(b, &m)
	for tag := range m {
		v.Unrecognized = tag
	}
	return nil
}

// ConfigValue: exactly one field is set.
type ConfigValue struct {
	String       *string `json:"-"`
	Bool         *bool   `json:"-"`
	Unrecognized string  `json:"-"` // a variant this package does not know (a newer binary): its wire name
}

// Name is the variant's wire name: "string", …
func (v ConfigValue) Name() string {
	switch {
	case v.String != nil:
		return "string"
	case v.Bool != nil:
		return "bool"
	}
	return v.Unrecognized
}

func (v ConfigValue) MarshalJSON() ([]byte, error) {
	switch {
	case v.String != nil:
		return json.Marshal(v.String)
	case v.Bool != nil:
		return json.Marshal(v.Bool)
	case v.Unrecognized != "":
		return json.Marshal(v.Unrecognized)
	}
	return nil, fmt.Errorf("ConfigValue: no variant set")
}

func (v *ConfigValue) UnmarshalJSON(b []byte) error {
	var s string
	if json.Unmarshal(b, &s) == nil {
		v.String = &s
		return nil
	}
	var raw bool
	if json.Unmarshal(b, &raw) == nil {
		v.Bool = &raw
		return nil
	}
	v.Unrecognized = string(b)
	return nil
}

// Answer: exactly one field is set.
type Answer struct {
	Permission   *PermissionChoice `json:"Permission"`
	Question     []QuestionAnswer  `json:"Question"`
	Unrecognized string            `json:"-"` // a variant this package does not know (a newer binary): its wire name
}

// Name is the variant's wire name: "Permission", …
func (v Answer) Name() string {
	switch {
	case v.Permission != nil:
		return "Permission"
	case v.Question != nil:
		return "Question"
	}
	return v.Unrecognized
}

func (v Answer) MarshalJSON() ([]byte, error) {
	switch {
	case v.Permission != nil:
		return json.Marshal(map[string]any{"Permission": v.Permission})
	case v.Question != nil:
		return json.Marshal(map[string]any{"Question": v.Question})
	case v.Unrecognized != "":
		return json.Marshal(v.Unrecognized)
	}
	return nil, fmt.Errorf("Answer: no variant set")
}

func (v *Answer) UnmarshalJSON(b []byte) error {
	var s string
	if json.Unmarshal(b, &s) == nil {
		v.Unrecognized = s
		return nil
	}
	type plain Answer
	if err := json.Unmarshal(b, (*plain)(v)); err != nil || v.Name() != "" {
		return err
	}
	var m map[string]json.RawMessage
	json.Unmarshal(b, &m)
	for tag := range m {
		v.Unrecognized = tag
	}
	return nil
}

// PermissionChoice is a wire type.
type PermissionChoice string

const (
	PermissionChoiceAllowOnce   PermissionChoice = "AllowOnce"
	PermissionChoiceAllowAlways PermissionChoice = "AllowAlways"
	PermissionChoiceDenyOnce    PermissionChoice = "DenyOnce"
	PermissionChoiceDenyAlways  PermissionChoice = "DenyAlways"
)

// QuestionAnswer: exactly one field is set.
type QuestionAnswer struct {
	Choices      []string `json:"Choices"`
	Text         *string  `json:"Text"`
	Unrecognized string   `json:"-"` // a variant this package does not know (a newer binary): its wire name
}

// Name is the variant's wire name: "Choices", …
func (v QuestionAnswer) Name() string {
	switch {
	case v.Choices != nil:
		return "Choices"
	case v.Text != nil:
		return "Text"
	}
	return v.Unrecognized
}

func (v QuestionAnswer) MarshalJSON() ([]byte, error) {
	switch {
	case v.Choices != nil:
		return json.Marshal(map[string]any{"Choices": v.Choices})
	case v.Text != nil:
		return json.Marshal(map[string]any{"Text": v.Text})
	case v.Unrecognized != "":
		return json.Marshal(v.Unrecognized)
	}
	return nil, fmt.Errorf("QuestionAnswer: no variant set")
}

func (v *QuestionAnswer) UnmarshalJSON(b []byte) error {
	var s string
	if json.Unmarshal(b, &s) == nil {
		v.Unrecognized = s
		return nil
	}
	type plain QuestionAnswer
	if err := json.Unmarshal(b, (*plain)(v)); err != nil || v.Name() != "" {
		return err
	}
	var m map[string]json.RawMessage
	json.Unmarshal(b, &m)
	for tag := range m {
		v.Unrecognized = tag
	}
	return nil
}

// RollbackScope: What `rollback` rewinds: conversation context only, or also the files
type RollbackScope string

const (
	RollbackScopeConversation         RollbackScope = "Conversation"
	RollbackScopeConversationAndFiles RollbackScope = "ConversationAndFiles"
)

// Event: One normalized event produced by anyagent.
type Event struct {
	Sequence   uint64         `json:"sequence"`
	OccurredAt *SystemTime    `json:"occurred_at,omitempty"`
	SessionID  string         `json:"session_id"`
	TurnInfo   *TurnContext   `json:"turn_info,omitempty"`
	Kind       EventKind      `json:"kind"`
	Extensions map[string]any `json:"extensions"`
}

// SystemTime is a wire type.
type SystemTime struct {
	SecsSinceEpoch  uint64 `json:"secs_since_epoch"`
	NanosSinceEpoch uint32 `json:"nanos_since_epoch"`
}

// TurnContext is a wire type.
type TurnContext struct {
	ID           string  `json:"id"`
	ParentToolID *string `json:"parent_tool_id,omitempty"`
}

// TurnStarted is a wire type.
type TurnStarted struct {
	Origin TurnOrigin `json:"origin"`
}

// TextDelta is a wire type.
type TextDelta struct {
	MessageID string `json:"message_id"`
	Text      string `json:"text"`
}

// ReasoningDelta is a wire type.
type ReasoningDelta struct {
	MessageID string `json:"message_id"`
	Text      string `json:"text"`
}

// UserMessage is a wire type.
type UserMessage struct {
	MessageID string `json:"message_id"`
	Text      string `json:"text"`
}

// MessageEnded is a wire type.
type MessageEnded struct {
	MessageID string `json:"message_id"`
}

// ToolOutputDelta is a wire type.
type ToolOutputDelta struct {
	ToolID string `json:"tool_id"`
	Text   string `json:"text"`
}

// PlanUpdated is a wire type.
type PlanUpdated struct {
	Entries []PlanEntry `json:"entries"`
}

// RequestClosed is a wire type.
type RequestClosed struct {
	RequestID string `json:"request_id"`
}

// ContextUsage is a wire type.
type ContextUsage struct {
	UsedTokens   uint64   `json:"used_tokens"`
	WindowTokens *uint64  `json:"window_tokens,omitempty"`
	CostUSD      *float64 `json:"cost_usd,omitempty"`
}

// TurnEnded is a wire type.
type TurnEnded struct {
	Stop       StopReason `json:"stop"`
	Background []string   `json:"background"`
}

// EventKind: exactly one field is set.
type EventKind struct {
	TurnStarted      *TurnStarted     `json:"TurnStarted"`
	TextDelta        *TextDelta       `json:"TextDelta"`
	ReasoningDelta   *ReasoningDelta  `json:"ReasoningDelta"`
	UserMessage      *UserMessage     `json:"UserMessage"`
	MessageEnded     *MessageEnded    `json:"MessageEnded"`
	ToolUpdated      *ToolUpdate      `json:"ToolUpdated"`
	ToolOutputDelta  *ToolOutputDelta `json:"ToolOutputDelta"`
	PlanUpdated      *PlanUpdated     `json:"PlanUpdated"`
	RequestOpened    *Request         `json:"RequestOpened"`
	RequestClosed    *RequestClosed   `json:"RequestClosed"`
	SessionUpdated   *SessionInfo     `json:"SessionUpdated"`
	StatusChanged    *SessionStatus   `json:"StatusChanged"`
	ContextUsage     *ContextUsage    `json:"ContextUsage"`
	ContextCompacted bool             `json:"-"`
	PlanUsageUpdated *PlanUsage       `json:"PlanUsageUpdated"`
	Diagnostic       *Diagnostic      `json:"Diagnostic"`
	TurnEnded        *TurnEnded       `json:"TurnEnded"`
	Unrecognized     string           `json:"-"` // a variant this package does not know (a newer binary): its wire name
}

// Name is the variant's wire name: "TurnStarted", …
func (v EventKind) Name() string {
	switch {
	case v.TurnStarted != nil:
		return "TurnStarted"
	case v.TextDelta != nil:
		return "TextDelta"
	case v.ReasoningDelta != nil:
		return "ReasoningDelta"
	case v.UserMessage != nil:
		return "UserMessage"
	case v.MessageEnded != nil:
		return "MessageEnded"
	case v.ToolUpdated != nil:
		return "ToolUpdated"
	case v.ToolOutputDelta != nil:
		return "ToolOutputDelta"
	case v.PlanUpdated != nil:
		return "PlanUpdated"
	case v.RequestOpened != nil:
		return "RequestOpened"
	case v.RequestClosed != nil:
		return "RequestClosed"
	case v.SessionUpdated != nil:
		return "SessionUpdated"
	case v.StatusChanged != nil:
		return "StatusChanged"
	case v.ContextUsage != nil:
		return "ContextUsage"
	case v.ContextCompacted:
		return "ContextCompacted"
	case v.PlanUsageUpdated != nil:
		return "PlanUsageUpdated"
	case v.Diagnostic != nil:
		return "Diagnostic"
	case v.TurnEnded != nil:
		return "TurnEnded"
	}
	return v.Unrecognized
}

func (v EventKind) MarshalJSON() ([]byte, error) {
	switch {
	case v.TurnStarted != nil:
		return json.Marshal(map[string]any{"TurnStarted": v.TurnStarted})
	case v.TextDelta != nil:
		return json.Marshal(map[string]any{"TextDelta": v.TextDelta})
	case v.ReasoningDelta != nil:
		return json.Marshal(map[string]any{"ReasoningDelta": v.ReasoningDelta})
	case v.UserMessage != nil:
		return json.Marshal(map[string]any{"UserMessage": v.UserMessage})
	case v.MessageEnded != nil:
		return json.Marshal(map[string]any{"MessageEnded": v.MessageEnded})
	case v.ToolUpdated != nil:
		return json.Marshal(map[string]any{"ToolUpdated": v.ToolUpdated})
	case v.ToolOutputDelta != nil:
		return json.Marshal(map[string]any{"ToolOutputDelta": v.ToolOutputDelta})
	case v.PlanUpdated != nil:
		return json.Marshal(map[string]any{"PlanUpdated": v.PlanUpdated})
	case v.RequestOpened != nil:
		return json.Marshal(map[string]any{"RequestOpened": v.RequestOpened})
	case v.RequestClosed != nil:
		return json.Marshal(map[string]any{"RequestClosed": v.RequestClosed})
	case v.SessionUpdated != nil:
		return json.Marshal(map[string]any{"SessionUpdated": v.SessionUpdated})
	case v.StatusChanged != nil:
		return json.Marshal(map[string]any{"StatusChanged": v.StatusChanged})
	case v.ContextUsage != nil:
		return json.Marshal(map[string]any{"ContextUsage": v.ContextUsage})
	case v.ContextCompacted:
		return json.Marshal("ContextCompacted")
	case v.PlanUsageUpdated != nil:
		return json.Marshal(map[string]any{"PlanUsageUpdated": v.PlanUsageUpdated})
	case v.Diagnostic != nil:
		return json.Marshal(map[string]any{"Diagnostic": v.Diagnostic})
	case v.TurnEnded != nil:
		return json.Marshal(map[string]any{"TurnEnded": v.TurnEnded})
	case v.Unrecognized != "":
		return json.Marshal(v.Unrecognized)
	}
	return nil, fmt.Errorf("EventKind: no variant set")
}

func (v *EventKind) UnmarshalJSON(b []byte) error {
	var s string
	if json.Unmarshal(b, &s) == nil {
		switch s {
		case "ContextCompacted":
			v.ContextCompacted = true
			return nil
		}
		v.Unrecognized = s
		return nil
	}
	type plain EventKind
	if err := json.Unmarshal(b, (*plain)(v)); err != nil || v.Name() != "" {
		return err
	}
	var m map[string]json.RawMessage
	json.Unmarshal(b, &m)
	for tag := range m {
		v.Unrecognized = tag
	}
	return nil
}

// TurnOrigin: exactly one field is set.
type TurnOrigin struct {
	Agent        bool    `json:"-"`
	Prompt       *string `json:"Prompt"`
	Unrecognized string  `json:"-"` // a variant this package does not know (a newer binary): its wire name
}

// Name is the variant's wire name: "Agent", …
func (v TurnOrigin) Name() string {
	switch {
	case v.Agent:
		return "Agent"
	case v.Prompt != nil:
		return "Prompt"
	}
	return v.Unrecognized
}

func (v TurnOrigin) MarshalJSON() ([]byte, error) {
	switch {
	case v.Agent:
		return json.Marshal("Agent")
	case v.Prompt != nil:
		return json.Marshal(map[string]any{"Prompt": v.Prompt})
	case v.Unrecognized != "":
		return json.Marshal(v.Unrecognized)
	}
	return nil, fmt.Errorf("TurnOrigin: no variant set")
}

func (v *TurnOrigin) UnmarshalJSON(b []byte) error {
	var s string
	if json.Unmarshal(b, &s) == nil {
		switch s {
		case "Agent":
			v.Agent = true
			return nil
		}
		v.Unrecognized = s
		return nil
	}
	type plain TurnOrigin
	if err := json.Unmarshal(b, (*plain)(v)); err != nil || v.Name() != "" {
		return err
	}
	var m map[string]json.RawMessage
	json.Unmarshal(b, &m)
	for tag := range m {
		v.Unrecognized = tag
	}
	return nil
}

// ToolUpdate: Cumulative snapshot of one tool call.
type ToolUpdate struct {
	ID        string     `json:"id"`
	Kind      ToolKind   `json:"kind"`
	Title     string     `json:"title"`
	Status    ToolStatus `json:"status"`
	Input     ToolInput  `json:"input"`
	Output    *string    `json:"output,omitempty"`
	Diffs     []FileDiff `json:"diffs"`
	Locations []string   `json:"locations"`
	Raw       *RawTool   `json:"raw,omitempty"`
}

// Mcp is a wire type.
type Mcp struct {
	Server string `json:"server"`
	Tool   string `json:"tool"`
}

// ToolKind: exactly one field is set.
type ToolKind struct {
	Read         bool   `json:"-"`
	Edit         bool   `json:"-"`
	Delete       bool   `json:"-"`
	Move         bool   `json:"-"`
	Search       bool   `json:"-"`
	Execute      bool   `json:"-"`
	Fetch        bool   `json:"-"`
	Think        bool   `json:"-"`
	Other        bool   `json:"-"`
	Mcp          *Mcp   `json:"Mcp"`
	Subagent     bool   `json:"-"`
	Unrecognized string `json:"-"` // a variant this package does not know (a newer binary): its wire name
}

// Name is the variant's wire name: "Read", …
func (v ToolKind) Name() string {
	switch {
	case v.Read:
		return "Read"
	case v.Edit:
		return "Edit"
	case v.Delete:
		return "Delete"
	case v.Move:
		return "Move"
	case v.Search:
		return "Search"
	case v.Execute:
		return "Execute"
	case v.Fetch:
		return "Fetch"
	case v.Think:
		return "Think"
	case v.Other:
		return "Other"
	case v.Mcp != nil:
		return "Mcp"
	case v.Subagent:
		return "Subagent"
	}
	return v.Unrecognized
}

func (v ToolKind) MarshalJSON() ([]byte, error) {
	switch {
	case v.Read:
		return json.Marshal("Read")
	case v.Edit:
		return json.Marshal("Edit")
	case v.Delete:
		return json.Marshal("Delete")
	case v.Move:
		return json.Marshal("Move")
	case v.Search:
		return json.Marshal("Search")
	case v.Execute:
		return json.Marshal("Execute")
	case v.Fetch:
		return json.Marshal("Fetch")
	case v.Think:
		return json.Marshal("Think")
	case v.Other:
		return json.Marshal("Other")
	case v.Mcp != nil:
		return json.Marshal(map[string]any{"Mcp": v.Mcp})
	case v.Subagent:
		return json.Marshal("Subagent")
	case v.Unrecognized != "":
		return json.Marshal(v.Unrecognized)
	}
	return nil, fmt.Errorf("ToolKind: no variant set")
}

func (v *ToolKind) UnmarshalJSON(b []byte) error {
	var s string
	if json.Unmarshal(b, &s) == nil {
		switch s {
		case "Read":
			v.Read = true
			return nil
		case "Edit":
			v.Edit = true
			return nil
		case "Delete":
			v.Delete = true
			return nil
		case "Move":
			v.Move = true
			return nil
		case "Search":
			v.Search = true
			return nil
		case "Execute":
			v.Execute = true
			return nil
		case "Fetch":
			v.Fetch = true
			return nil
		case "Think":
			v.Think = true
			return nil
		case "Other":
			v.Other = true
			return nil
		case "Subagent":
			v.Subagent = true
			return nil
		}
		v.Unrecognized = s
		return nil
	}
	type plain ToolKind
	if err := json.Unmarshal(b, (*plain)(v)); err != nil || v.Name() != "" {
		return err
	}
	var m map[string]json.RawMessage
	json.Unmarshal(b, &m)
	for tag := range m {
		v.Unrecognized = tag
	}
	return nil
}

// ToolStatus is a wire type.
type ToolStatus string

const (
	ToolStatusPending   ToolStatus = "Pending"
	ToolStatusRunning   ToolStatus = "Running"
	ToolStatusCompleted ToolStatus = "Completed"
	ToolStatusFailed    ToolStatus = "Failed"
	ToolStatusCancelled ToolStatus = "Cancelled"
)

// Command is a wire type.
type Command struct {
	Command string  `json:"command"`
	Cwd     *string `json:"cwd,omitempty"`
}

// ToolInput: exactly one field is set.
type ToolInput struct {
	None         bool     `json:"-"`
	Path         *string  `json:"Path"`
	Command      *Command `json:"Command"`
	Pattern      *string  `json:"Pattern"`
	Url          *string  `json:"Url"`
	Query        *string  `json:"Query"`
	Text         *string  `json:"Text"`
	Unrecognized string   `json:"-"` // a variant this package does not know (a newer binary): its wire name
}

// Name is the variant's wire name: "None", …
func (v ToolInput) Name() string {
	switch {
	case v.None:
		return "None"
	case v.Path != nil:
		return "Path"
	case v.Command != nil:
		return "Command"
	case v.Pattern != nil:
		return "Pattern"
	case v.Url != nil:
		return "Url"
	case v.Query != nil:
		return "Query"
	case v.Text != nil:
		return "Text"
	}
	return v.Unrecognized
}

func (v ToolInput) MarshalJSON() ([]byte, error) {
	switch {
	case v.None:
		return json.Marshal("None")
	case v.Path != nil:
		return json.Marshal(map[string]any{"Path": v.Path})
	case v.Command != nil:
		return json.Marshal(map[string]any{"Command": v.Command})
	case v.Pattern != nil:
		return json.Marshal(map[string]any{"Pattern": v.Pattern})
	case v.Url != nil:
		return json.Marshal(map[string]any{"Url": v.Url})
	case v.Query != nil:
		return json.Marshal(map[string]any{"Query": v.Query})
	case v.Text != nil:
		return json.Marshal(map[string]any{"Text": v.Text})
	case v.Unrecognized != "":
		return json.Marshal(v.Unrecognized)
	}
	return nil, fmt.Errorf("ToolInput: no variant set")
}

func (v *ToolInput) UnmarshalJSON(b []byte) error {
	var s string
	if json.Unmarshal(b, &s) == nil {
		switch s {
		case "None":
			v.None = true
			return nil
		}
		v.Unrecognized = s
		return nil
	}
	type plain ToolInput
	if err := json.Unmarshal(b, (*plain)(v)); err != nil || v.Name() != "" {
		return err
	}
	var m map[string]json.RawMessage
	json.Unmarshal(b, &m)
	for tag := range m {
		v.Unrecognized = tag
	}
	return nil
}

// FileDiff is a wire type.
type FileDiff struct {
	Path    string  `json:"path"`
	OldText *string `json:"old_text,omitempty"`
	NewText string  `json:"new_text"`
}

// RawTool is a wire type.
type RawTool struct {
	Name  string `json:"name"`
	Input any    `json:"input"`
}

// PlanEntry is a wire type.
type PlanEntry struct {
	Text   string     `json:"text"`
	Status PlanStatus `json:"status"`
}

// PlanStatus is a wire type.
type PlanStatus string

const (
	PlanStatusPending    PlanStatus = "Pending"
	PlanStatusInProgress PlanStatus = "InProgress"
	PlanStatusCompleted  PlanStatus = "Completed"
)

// Request: exactly one field is set. Something the agent is waiting on the caller for. Answer once with
type Request struct {
	Permission   *PermissionRequest `json:"Permission"`
	Question     *QuestionRequest   `json:"Question"`
	Unrecognized string             `json:"-"` // a variant this package does not know (a newer binary): its wire name
}

// Name is the variant's wire name: "Permission", …
func (v Request) Name() string {
	switch {
	case v.Permission != nil:
		return "Permission"
	case v.Question != nil:
		return "Question"
	}
	return v.Unrecognized
}

func (v Request) MarshalJSON() ([]byte, error) {
	switch {
	case v.Permission != nil:
		return json.Marshal(map[string]any{"Permission": v.Permission})
	case v.Question != nil:
		return json.Marshal(map[string]any{"Question": v.Question})
	case v.Unrecognized != "":
		return json.Marshal(v.Unrecognized)
	}
	return nil, fmt.Errorf("Request: no variant set")
}

func (v *Request) UnmarshalJSON(b []byte) error {
	var s string
	if json.Unmarshal(b, &s) == nil {
		v.Unrecognized = s
		return nil
	}
	type plain Request
	if err := json.Unmarshal(b, (*plain)(v)); err != nil || v.Name() != "" {
		return err
	}
	var m map[string]json.RawMessage
	json.Unmarshal(b, &m)
	for tag := range m {
		v.Unrecognized = tag
	}
	return nil
}

// PermissionRequest is a wire type.
type PermissionRequest struct {
	ID      string             `json:"id"`
	Tool    ToolUpdate         `json:"tool"`
	Options []PermissionChoice `json:"options"`
	Detail  *string            `json:"detail,omitempty"`
}

// QuestionRequest is a wire type.
type QuestionRequest struct {
	ID        string     `json:"id"`
	Questions []Question `json:"questions"`
}

// Question is a wire type.
type Question struct {
	ID             string   `json:"id"`
	Text           string   `json:"text"`
	Header         *string  `json:"header,omitempty"`
	Choices        []Choice `json:"choices"`
	MultiSelect    bool     `json:"multi_select"`
	AllowsFreeText bool     `json:"allows_free_text"`
}

// Choice is a wire type.
type Choice struct {
	ID          string  `json:"id"`
	Label       string  `json:"label"`
	Description *string `json:"description,omitempty"`
}

// SessionInfo: Snapshot of a live session. Also carried by `EventKind::SessionUpdated`.
type SessionInfo struct {
	ID            string               `json:"id"`
	Agent         AgentInstallation    `json:"agent"`
	Details       AgentDetails         `json:"details"`
	Configuration SessionConfiguration `json:"configuration"`
	ResumeToken   *string              `json:"resume_token,omitempty"`
	Title         *string              `json:"title,omitempty"`
	Status        *SessionStatus       `json:"status,omitempty"`
}

// AgentInstallation: One installed agent, as returned by `Runtime::discover`.
type AgentInstallation struct {
	ID             string             `json:"id"`
	Name           string             `json:"name"`
	ExecutablePath string             `json:"executable_path"`
	Source         InstallationSource `json:"source"`
	Upgrade        *MissingAgent      `json:"upgrade,omitempty"`
	AcpArgs        []string           `json:"acp_args,omitempty"`
}

// InstallationSource: Where discovery found an executable.
type InstallationSource string

const (
	InstallationSourceEnvOverride    InstallationSource = "EnvOverride"
	InstallationSourcePath           InstallationSource = "Path"
	InstallationSourceLoginShellPath InstallationSource = "LoginShellPath"
	InstallationSourceVersionManager InstallationSource = "VersionManager"
	InstallationSourceKnownLocation  InstallationSource = "KnownLocation"
	InstallationSourcePinned         InstallationSource = "Pinned"
)

// MissingAgent is a wire type.
type MissingAgent struct {
	ID          string   `json:"id"`
	Name        string   `json:"name"`
	Searched    []string `json:"searched"`
	InstallHint string   `json:"install_hint"`
}

// AgentDetails: What `probe` and `open` learn about an agent.
type AgentDetails struct {
	Version       *string        `json:"version,omitempty"`
	Auth          AuthStatus     `json:"auth"`
	Capabilities  Capabilities   `json:"capabilities"`
	ConfigOptions []ConfigOption `json:"config_options"`
	Commands      []SlashCommand `json:"commands"`
}

// Authenticated is a wire type.
type Authenticated struct {
	Kind    AuthKind     `json:"kind"`
	Account *AccountInfo `json:"account,omitempty"`
}

// Unauthenticated is a wire type.
type Unauthenticated struct {
	Login []LoginMethod `json:"login"`
}

// AuthStatus: exactly one field is set. Whether, and how, an agent is logged in.
type AuthStatus struct {
	Unknown         bool             `json:"-"`
	Authenticated   *Authenticated   `json:"Authenticated"`
	Unauthenticated *Unauthenticated `json:"Unauthenticated"`
	Unrecognized    string           `json:"-"` // a variant this package does not know (a newer binary): its wire name
}

// Name is the variant's wire name: "Unknown", …
func (v AuthStatus) Name() string {
	switch {
	case v.Unknown:
		return "Unknown"
	case v.Authenticated != nil:
		return "Authenticated"
	case v.Unauthenticated != nil:
		return "Unauthenticated"
	}
	return v.Unrecognized
}

func (v AuthStatus) MarshalJSON() ([]byte, error) {
	switch {
	case v.Unknown:
		return json.Marshal("Unknown")
	case v.Authenticated != nil:
		return json.Marshal(map[string]any{"Authenticated": v.Authenticated})
	case v.Unauthenticated != nil:
		return json.Marshal(map[string]any{"Unauthenticated": v.Unauthenticated})
	case v.Unrecognized != "":
		return json.Marshal(v.Unrecognized)
	}
	return nil, fmt.Errorf("AuthStatus: no variant set")
}

func (v *AuthStatus) UnmarshalJSON(b []byte) error {
	var s string
	if json.Unmarshal(b, &s) == nil {
		switch s {
		case "Unknown":
			v.Unknown = true
			return nil
		}
		v.Unrecognized = s
		return nil
	}
	type plain AuthStatus
	if err := json.Unmarshal(b, (*plain)(v)); err != nil || v.Name() != "" {
		return err
	}
	var m map[string]json.RawMessage
	json.Unmarshal(b, &m)
	for tag := range m {
		v.Unrecognized = tag
	}
	return nil
}

// AuthKind: exactly one field is set. The login kind decides which features exist (plan usage needs a subscription).
type AuthKind struct {
	Subscription  bool    `json:"-"`
	ApiKey        bool    `json:"-"`
	CloudProvider bool    `json:"-"`
	Other         *string `json:"Other"`
	Unrecognized  string  `json:"-"` // a variant this package does not know (a newer binary): its wire name
}

// Name is the variant's wire name: "Subscription", …
func (v AuthKind) Name() string {
	switch {
	case v.Subscription:
		return "Subscription"
	case v.ApiKey:
		return "ApiKey"
	case v.CloudProvider:
		return "CloudProvider"
	case v.Other != nil:
		return "Other"
	}
	return v.Unrecognized
}

func (v AuthKind) MarshalJSON() ([]byte, error) {
	switch {
	case v.Subscription:
		return json.Marshal("Subscription")
	case v.ApiKey:
		return json.Marshal("ApiKey")
	case v.CloudProvider:
		return json.Marshal("CloudProvider")
	case v.Other != nil:
		return json.Marshal(map[string]any{"Other": v.Other})
	case v.Unrecognized != "":
		return json.Marshal(v.Unrecognized)
	}
	return nil, fmt.Errorf("AuthKind: no variant set")
}

func (v *AuthKind) UnmarshalJSON(b []byte) error {
	var s string
	if json.Unmarshal(b, &s) == nil {
		switch s {
		case "Subscription":
			v.Subscription = true
			return nil
		case "ApiKey":
			v.ApiKey = true
			return nil
		case "CloudProvider":
			v.CloudProvider = true
			return nil
		}
		v.Unrecognized = s
		return nil
	}
	type plain AuthKind
	if err := json.Unmarshal(b, (*plain)(v)); err != nil || v.Name() != "" {
		return err
	}
	var m map[string]json.RawMessage
	json.Unmarshal(b, &m)
	for tag := range m {
		v.Unrecognized = tag
	}
	return nil
}

// AccountInfo is a wire type.
type AccountInfo struct {
	Email *string `json:"email,omitempty"`
	Plan  *string `json:"plan,omitempty"`
}

// Terminal is a wire type.
type Terminal struct {
	Command     []string          `json:"command"`
	Env         map[string]string `json:"env"`
	Description string            `json:"description"`
}

// EnvVar is a wire type.
type EnvVar struct {
	Name string `json:"name"`
}

// LoginMethod: exactly one field is set. A login method the application can show to the user.
type LoginMethod struct {
	Terminal     *Terminal `json:"Terminal"`
	EnvVar       *EnvVar   `json:"EnvVar"`
	Unrecognized string    `json:"-"` // a variant this package does not know (a newer binary): its wire name
}

// Name is the variant's wire name: "Terminal", …
func (v LoginMethod) Name() string {
	switch {
	case v.Terminal != nil:
		return "Terminal"
	case v.EnvVar != nil:
		return "EnvVar"
	}
	return v.Unrecognized
}

func (v LoginMethod) MarshalJSON() ([]byte, error) {
	switch {
	case v.Terminal != nil:
		return json.Marshal(map[string]any{"Terminal": v.Terminal})
	case v.EnvVar != nil:
		return json.Marshal(map[string]any{"EnvVar": v.EnvVar})
	case v.Unrecognized != "":
		return json.Marshal(v.Unrecognized)
	}
	return nil, fmt.Errorf("LoginMethod: no variant set")
}

func (v *LoginMethod) UnmarshalJSON(b []byte) error {
	var s string
	if json.Unmarshal(b, &s) == nil {
		v.Unrecognized = s
		return nil
	}
	type plain LoginMethod
	if err := json.Unmarshal(b, (*plain)(v)); err != nil || v.Name() != "" {
		return err
	}
	var m map[string]json.RawMessage
	json.Unmarshal(b, &m)
	for tag := range m {
		v.Unrecognized = tag
	}
	return nil
}

// Capabilities: Effective caller actions for one agent or session.
type Capabilities struct {
	Features      []Capability   `json:"features"`
	McpTransports []McpTransport `json:"mcp_transports"`
}

// Capability: Optional actions supported by an agent or session.
type Capability string

const (
	CapabilityImages        Capability = "Images"
	CapabilityResume        Capability = "Resume"
	CapabilitySteer         Capability = "Steer"
	CapabilityPermissions   Capability = "Permissions"
	CapabilityQuestions     Capability = "Questions"
	CapabilityRollback      Capability = "Rollback"
	CapabilityFork          Capability = "Fork"
	CapabilitySlashCommands Capability = "SlashCommands"
	CapabilityPlan          Capability = "Plan"
	CapabilitySubagents     Capability = "Subagents"
	CapabilityContextUsage  Capability = "ContextUsage"
	CapabilityPlanUsage     Capability = "PlanUsage"
	CapabilityRollbackFiles Capability = "RollbackFiles"
	CapabilityCompact       Capability = "Compact"
)

// McpTransport is a wire type.
type McpTransport string

const (
	McpTransportStdio McpTransport = "Stdio"
	McpTransportHttp  McpTransport = "Http"
	McpTransportSse   McpTransport = "Sse"
)

// ConfigOption: A session setting the agent advertises. Well-known ids: `model`, `effort`,
type ConfigOption struct {
	ID       string       `json:"id"`
	Name     string       `json:"name"`
	Category *string      `json:"category,omitempty"`
	Kind     ConfigKind   `json:"kind"`
	Current  *ConfigValue `json:"current,omitempty"`
	Live     bool         `json:"live"`
}

// Select is a wire type.
type Select struct {
	Choices []ConfigChoice `json:"choices"`
}

// ConfigKind: exactly one field is set.
type ConfigKind struct {
	Boolean      bool    `json:"-"`
	Select       *Select `json:"Select"`
	Unrecognized string  `json:"-"` // a variant this package does not know (a newer binary): its wire name
}

// Name is the variant's wire name: "Boolean", …
func (v ConfigKind) Name() string {
	switch {
	case v.Boolean:
		return "Boolean"
	case v.Select != nil:
		return "Select"
	}
	return v.Unrecognized
}

func (v ConfigKind) MarshalJSON() ([]byte, error) {
	switch {
	case v.Boolean:
		return json.Marshal("Boolean")
	case v.Select != nil:
		return json.Marshal(map[string]any{"Select": v.Select})
	case v.Unrecognized != "":
		return json.Marshal(v.Unrecognized)
	}
	return nil, fmt.Errorf("ConfigKind: no variant set")
}

func (v *ConfigKind) UnmarshalJSON(b []byte) error {
	var s string
	if json.Unmarshal(b, &s) == nil {
		switch s {
		case "Boolean":
			v.Boolean = true
			return nil
		}
		v.Unrecognized = s
		return nil
	}
	type plain ConfigKind
	if err := json.Unmarshal(b, (*plain)(v)); err != nil || v.Name() != "" {
		return err
	}
	var m map[string]json.RawMessage
	json.Unmarshal(b, &m)
	for tag := range m {
		v.Unrecognized = tag
	}
	return nil
}

// ConfigChoice is a wire type.
type ConfigChoice struct {
	Value       string  `json:"value"`
	Label       string  `json:"label"`
	Description *string `json:"description,omitempty"`
}

// SlashCommand is a wire type.
type SlashCommand struct {
	Name        string  `json:"name"`
	Description string  `json:"description"`
	InputHint   *string `json:"input_hint,omitempty"`
}

// SessionConfiguration is a wire type.
type SessionConfiguration struct {
	Options map[string]ConfigValue `json:"options"`
}

// SessionStatus: What a UI should show for the session right now. Changes arrive as
type SessionStatus string

const (
	SessionStatusIdle       SessionStatus = "Idle"
	SessionStatusWorking    SessionStatus = "Working"
	SessionStatusNeedsInput SessionStatus = "NeedsInput"
)

// PlanUsage: Plan quota windows for the logged-in account.
type PlanUsage struct {
	Plan      *string       `json:"plan,omitempty"`
	Windows   []UsageWindow `json:"windows"`
	FetchedAt SystemTime    `json:"fetched_at"`
}

// UsageWindow is a wire type.
type UsageWindow struct {
	Label       string      `json:"label"`
	UsedPercent uint8       `json:"used_percent"`
	ResetsAt    *SystemTime `json:"resets_at,omitempty"`
}

// Diagnostic is a wire type.
type Diagnostic struct {
	Level   DiagnosticLevel `json:"level"`
	Message string          `json:"message"`
}

// DiagnosticLevel is a wire type.
type DiagnosticLevel string

const (
	DiagnosticLevelInfo    DiagnosticLevel = "Info"
	DiagnosticLevelWarning DiagnosticLevel = "Warning"
	DiagnosticLevelError   DiagnosticLevel = "Error"
)

// Completed is a wire type.
type Completed struct {
	Source CompletionSource `json:"source"`
}

// Failed is a wire type.
type Failed struct {
	Message string `json:"message"`
}

// StopReason: exactly one field is set.
type StopReason struct {
	Cancelled    bool       `json:"-"`
	Refused      bool       `json:"-"`
	Completed    *Completed `json:"Completed"`
	Failed       *Failed    `json:"Failed"`
	Unrecognized string     `json:"-"` // a variant this package does not know (a newer binary): its wire name
}

// Name is the variant's wire name: "Cancelled", …
func (v StopReason) Name() string {
	switch {
	case v.Cancelled:
		return "Cancelled"
	case v.Refused:
		return "Refused"
	case v.Completed != nil:
		return "Completed"
	case v.Failed != nil:
		return "Failed"
	}
	return v.Unrecognized
}

func (v StopReason) MarshalJSON() ([]byte, error) {
	switch {
	case v.Cancelled:
		return json.Marshal("Cancelled")
	case v.Refused:
		return json.Marshal("Refused")
	case v.Completed != nil:
		return json.Marshal(map[string]any{"Completed": v.Completed})
	case v.Failed != nil:
		return json.Marshal(map[string]any{"Failed": v.Failed})
	case v.Unrecognized != "":
		return json.Marshal(v.Unrecognized)
	}
	return nil, fmt.Errorf("StopReason: no variant set")
}

func (v *StopReason) UnmarshalJSON(b []byte) error {
	var s string
	if json.Unmarshal(b, &s) == nil {
		switch s {
		case "Cancelled":
			v.Cancelled = true
			return nil
		case "Refused":
			v.Refused = true
			return nil
		}
		v.Unrecognized = s
		return nil
	}
	type plain StopReason
	if err := json.Unmarshal(b, (*plain)(v)); err != nil || v.Name() != "" {
		return err
	}
	var m map[string]json.RawMessage
	json.Unmarshal(b, &m)
	for tag := range m {
		v.Unrecognized = tag
	}
	return nil
}

// CompletionSource is a wire type.
type CompletionSource string

const (
	CompletionSourceProtocol CompletionSource = "Protocol"
	CompletionSourceInferred CompletionSource = "Inferred"
)

// DiscoveryReport: What `discover` found and what it could not read.
type DiscoveryReport struct {
	Agents      []AgentInstallation `json:"agents"`
	Missing     []MissingAgent      `json:"missing"`
	Diagnostics []Diagnostic        `json:"diagnostics"`
}

// Delivery: Immediate result of submitting a prompt.
type Delivery struct {
	PromptID string       `json:"prompt_id"`
	Kind     DeliveryKind `json:"kind"`
}

// Started is a wire type.
type Started struct {
	TurnID string `json:"turn_id"`
}

// Steered is a wire type.
type Steered struct {
	TurnID string `json:"turn_id"`
}

// Queued is a wire type.
type Queued struct {
	Position uint32 `json:"position"`
}

// DeliveryKind: exactly one field is set.
type DeliveryKind struct {
	Started      *Started `json:"Started"`
	Steered      *Steered `json:"Steered"`
	Queued       *Queued  `json:"Queued"`
	Unrecognized string   `json:"-"` // a variant this package does not know (a newer binary): its wire name
}

// Name is the variant's wire name: "Started", …
func (v DeliveryKind) Name() string {
	switch {
	case v.Started != nil:
		return "Started"
	case v.Steered != nil:
		return "Steered"
	case v.Queued != nil:
		return "Queued"
	}
	return v.Unrecognized
}

func (v DeliveryKind) MarshalJSON() ([]byte, error) {
	switch {
	case v.Started != nil:
		return json.Marshal(map[string]any{"Started": v.Started})
	case v.Steered != nil:
		return json.Marshal(map[string]any{"Steered": v.Steered})
	case v.Queued != nil:
		return json.Marshal(map[string]any{"Queued": v.Queued})
	case v.Unrecognized != "":
		return json.Marshal(v.Unrecognized)
	}
	return nil, fmt.Errorf("DeliveryKind: no variant set")
}

func (v *DeliveryKind) UnmarshalJSON(b []byte) error {
	var s string
	if json.Unmarshal(b, &s) == nil {
		v.Unrecognized = s
		return nil
	}
	type plain DeliveryKind
	if err := json.Unmarshal(b, (*plain)(v)); err != nil || v.Name() != "" {
		return err
	}
	var m map[string]json.RawMessage
	json.Unmarshal(b, &m)
	for tag := range m {
		v.Unrecognized = tag
	}
	return nil
}
