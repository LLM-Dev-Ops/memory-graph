# CLI Commands Reference

Complete reference for all LLM Memory Graph CLI commands.

## Command Structure

```
llm-memory-graph [GLOBAL_OPTIONS] <COMMAND> [COMMAND_OPTIONS] [ARGS]
```

## Global Options

| Option | Short | Type | Default | Description |
|--------|-------|------|---------|-------------|
| `--db-path` | `-d` | Path | `./data` | Path to database directory |
| `--format` | `-f` | String | `text` | Output format (text/json/yaml/table) |
| `--help` | `-h` | Flag | - | Print help information |
| `--version` | `-V` | Flag | - | Print version information |

## Commands

### stats

Display database statistics and metrics.

**Usage:**
```bash
llm-memory-graph stats
```

**Output Fields:**
- `total_nodes` - Total number of nodes
- `total_edges` - Total number of edges
- `total_sessions` - Total sessions created
- `active_sessions` - Currently active sessions
- `node_types` - Breakdown by node type
- `storage_size` - Database storage size

**Example:**
```bash
$ llm-memory-graph stats -f json
{
  "total_nodes": 1523,
  "total_edges": 2047,
  "total_sessions": 45,
  "active_sessions": 3,
  "node_types": {
    "prompt": 512,
    "response": 498,
    "tool_invocation": 413,
    "template": 67,
    "agent": 33
  }
}
```

---

### session

Session management commands.

#### session get

Get detailed information about a session.

**Usage:**
```bash
llm-memory-graph session get <SESSION_ID>
```

**Arguments:**
- `SESSION_ID` (required) - UUID of the session

**Output Fields:**
- `id` - Session UUID
- `created_at` - Creation timestamp
- `updated_at` - Last update timestamp
- `is_active` - Active status
- `metadata` - Session metadata
- `node_count` - Number of nodes in session

**Example:**
```bash
$ llm-memory-graph session get 550e8400-e29b-41d4-a716-446655440000
Session: 550e8400-e29b-41d4-a716-446655440000
Created: 2024-01-15T10:30:00Z
Updated: 2024-01-15T11:45:30Z
Active: true
Nodes: 47
Metadata:
  user: john_doe
  project: chatbot-v2
```

---

### node

Node operations.

#### node get

Get detailed information about a node.

**Usage:**
```bash
llm-memory-graph node get <NODE_ID>
```

**Arguments:**
- `NODE_ID` (required) - UUID of the node

**Output Fields:**
- `id` - Node UUID
- `type` - Node type
- `created_at` - Creation timestamp
- `data` - Node-specific data
- `edges` - Connected edges

**Example:**
```bash
$ llm-memory-graph node get 123e4567-e89b-12d3-a456-426614174000 -f json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "type": "prompt",
  "created_at": "2024-01-15T10:35:22Z",
  "data": {
    "content": "What is the weather today?",
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "metadata": {
      "model": "gpt-4",
      "temperature": 0.7
    }
  }
}
```

---

### query

Advanced query with multiple filters.

**Usage:**
```bash
llm-memory-graph query [OPTIONS]
```

**Options:**

| Option | Short | Type | Description |
|--------|-------|------|-------------|
| `--session` | `-s` | UUID | Filter by session ID |
| `--node-type` | `-t` | String | Filter by node type |
| `--after` | `-a` | RFC3339 | Filter nodes created after timestamp |
| `--before` | `-b` | RFC3339 | Filter nodes created before timestamp |
| `--limit` | `-l` | Number | Limit number of results |

**Node Types:**
- `prompt` - User prompts
- `response` - LLM responses
- `tool_invocation` - Tool invocations
- `agent` - AI agents
- `template` - Prompt templates

**Examples:**

```bash
# Get all prompts in a session
llm-memory-graph query -s 550e8400-e29b-41d4-a716-446655440000 -t prompt

# Get recent responses (last 24 hours)
llm-memory-graph query -t response -a 2024-01-15T00:00:00Z

# Get latest 20 nodes
llm-memory-graph query -l 20

# Complex query
llm-memory-graph query \
  -s 550e8400-e29b-41d4-a716-446655440000 \
  -t response \
  -a 2024-01-01T00:00:00Z \
  -b 2024-02-01T00:00:00Z \
  -l 50 \
  -f table
```

---

### export

Export data from the memory graph.

#### export session

Export a complete session.

**Usage:**
```bash
llm-memory-graph export session <SESSION_ID> -o <OUTPUT_FILE> [--export-format <FORMAT>]
```

**Arguments:**
- `SESSION_ID` (required) - Session UUID to export

**Options:**
- `-o, --output` (required) - Output file path
- `--export-format` - Export format (`json` or `msgpack`, default: `json`)

**Example:**
```bash
llm-memory-graph export session \
  550e8400-e29b-41d4-a716-446655440000 \
  -o session-backup.json

# Export to MessagePack
llm-memory-graph export session \
  550e8400-e29b-41d4-a716-446655440000 \
  -o session-backup.msgpack \
  --export-format msgpack
```

#### export database

Export the entire database.

**Usage:**
```bash
llm-memory-graph export database -o <OUTPUT_FILE> [--export-format <FORMAT>]
```

**Options:**
- `-o, --output` (required) - Output file path
- `--export-format` - Export format (`json` or `msgpack`, default: `json`)

**Example:**
```bash
llm-memory-graph export database -o full-backup.json

# Compressed export
llm-memory-graph export database -o backup.msgpack --export-format msgpack
```

---

### import

Import data into the memory graph.

**Usage:**
```bash
llm-memory-graph import -i <INPUT_FILE> [--import-format <FORMAT>] [--dry-run]
```

**Options:**
- `-i, --input` (required) - Input file path
- `--import-format` - Import format (`json` or `msgpack`, default: `json`)
- `--dry-run` - Validate without importing

**Examples:**

```bash
# Import from JSON
llm-memory-graph import -i backup.json

# Dry run (validation only)
llm-memory-graph import -i backup.json --dry-run

# Import MessagePack
llm-memory-graph import -i backup.msgpack --import-format msgpack
```

**Import Process:**
1. Validates file format
2. Checks for ID conflicts
3. Validates relationships
4. Imports nodes and edges
5. Updates indexes

**Conflict Resolution:**
- Existing IDs are skipped by default
- Use `--force` to overwrite (if implemented)
- Validation errors halt import

---

### template

Manage prompt templates.

#### template list

List all templates.

**Usage:**
```bash
llm-memory-graph template list
```

**Example:**
```bash
$ llm-memory-graph template list -f table
┌──────────────────────────────────────┬───────────┬─────────┬────────────┐
│ ID                                   │ Name      │ Version │ Usage      │
├──────────────────────────────────────┼───────────┼─────────┼────────────┤
│ a1b2c3d4-e5f6-7890-abcd-ef1234567890 │ greeting  │ 1       │ 145        │
│ b2c3d4e5-f6a7-8901-bcde-f12345678901 │ summary   │ 2       │ 89         │
└──────────────────────────────────────┴───────────┴─────────┴────────────┘
```

#### template get

Get template details.

**Usage:**
```bash
llm-memory-graph template get <TEMPLATE_ID>
```

**Arguments:**
- `TEMPLATE_ID` (required) - Template UUID

**Example:**
```bash
$ llm-memory-graph template get a1b2c3d4-e5f6-7890-abcd-ef1234567890
Template: greeting (v1)
ID: a1b2c3d4-e5f6-7890-abcd-ef1234567890
Created: 2024-01-10T09:00:00Z
Usage Count: 145

Text:
Hello {{name}}, welcome to {{location}}!

Variables:
  - name (string, required)
  - location (string, required, default: "our platform")
```

#### template create

Create a new template.

**Usage:**
```bash
llm-memory-graph template create --name <NAME> --text <TEMPLATE_TEXT> [--variables <VARS>]
```

**Options:**
- `--name` (required) - Template name
- `--text` (required) - Template text with variables
- `--variables` - Variable specifications (JSON format)

**Example:**
```bash
llm-memory-graph template create \
  --name "code_review" \
  --text "Review the following {{language}} code: {{code}}" \
  --variables '[{"name":"language","required":true},{"name":"code","required":true}]'
```

---

### agent

Manage AI agents.

#### agent list

List all agents.

**Usage:**
```bash
llm-memory-graph agent list
```

**Example:**
```bash
$ llm-memory-graph agent list -f table
┌──────────────────────────────────────┬──────────────┬─────────────┬────────┐
│ ID                                   │ Name         │ Role        │ Status │
├──────────────────────────────────────┼──────────────┼─────────────┼────────┤
│ c3d4e5f6-a7b8-9012-cdef-123456789012 │ CodeHelper   │ assistant   │ active │
│ d4e5f6a7-b8c9-0123-def1-234567890123 │ Researcher   │ researcher  │ active │
└──────────────────────────────────────┴──────────────┴─────────────┴────────┘
```

#### agent get

Get agent details.

**Usage:**
```bash
llm-memory-graph agent get <AGENT_ID>
```

**Arguments:**
- `AGENT_ID` (required) - Agent UUID

#### agent create

Create a new agent.

**Usage:**
```bash
llm-memory-graph agent create --name <NAME> --role <ROLE> [--capabilities <CAPS>]
```

**Options:**
- `--name` (required) - Agent name
- `--role` (required) - Agent role
- `--capabilities` - Comma-separated capabilities

**Example:**
```bash
llm-memory-graph agent create \
  --name "DataAnalyst" \
  --role "analyst" \
  --capabilities "python,sql,visualization"
```

---

### server

Manage the gRPC server.

#### server start

Start the gRPC server.

**Usage:**
```bash
llm-memory-graph server start [OPTIONS]
```

**Options:**
- `--host` - Server host (default: `0.0.0.0`)
- `--port` - Server port (default: `50051`)
- `--tls-cert` - TLS certificate file path
- `--tls-key` - TLS private key file path

**Example:**
```bash
# Start with defaults
llm-memory-graph server start

# Start with TLS
llm-memory-graph server start \
  --host 0.0.0.0 \
  --port 50051 \
  --tls-cert /etc/certs/server.crt \
  --tls-key /etc/certs/server.key
```

#### server stop

Stop a running server.

**Usage:**
```bash
llm-memory-graph server stop [--host <HOST>] [--port <PORT>]
```

**Options:**
- `--host` - Server host (default: `localhost`)
- `--port` - Server port (default: `50051`)

---

### flush

Flush database to disk.

**Usage:**
```bash
llm-memory-graph flush
```

**Description:**
Forces all pending database writes to be persisted to disk. Useful before:
- System shutdown
- Backup operations
- Database migrations

**Example:**
```bash
llm-memory-graph flush
Database flushed successfully
```

---

### verify

Verify database integrity.

**Usage:**
```bash
llm-memory-graph verify
```

**Checks:**
- Node reference integrity
- Edge endpoint validation
- Session consistency
- Index correctness
- Storage consistency

**Example:**
```bash
$ llm-memory-graph verify
Verifying database integrity...
✓ Nodes: 1523 (all valid)
✓ Edges: 2047 (all valid)
✓ Sessions: 45 (all valid)
✓ References: all consistent
✓ Indexes: up to date

Database verification complete. No issues found.
```

## Error Handling

### Common Errors

**Invalid UUID Format:**
```
Error: Invalid UUID format for session_id
Expected: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
```

**Database Not Found:**
```
Error: Database not found at path: ./data
Use --db-path to specify the correct location
```

**Permission Denied:**
```
Error: Permission denied accessing database at ./data
Check file permissions and ownership
```

**Database Locked:**
```
Error: Database is locked by another process
Ensure no other instances are running
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Database error |
| 4 | Network error |
| 5 | IO error |

## Advanced Usage

### Scripting

Use JSON output for scripting:

```bash
#!/bin/bash

# Get session count
SESSION_COUNT=$(llm-memory-graph stats -f json | jq '.total_sessions')

# Export all sessions
llm-memory-graph export database -o "backup-$(date +%Y%m%d).json"

# Verify database
if llm-memory-graph verify; then
  echo "Database healthy"
else
  echo "Database issues detected"
  exit 1
fi
```

### Batch Operations

```bash
# Export multiple sessions
for session_id in $(cat session-list.txt); do
  llm-memory-graph export session "$session_id" -o "session-${session_id}.json"
done

# Query and process
llm-memory-graph query -t prompt -f json | \
  jq '.nodes[] | select(.data.metadata.model == "gpt-4")'
```

### Pipeline Integration

```bash
# Export to cloud storage
llm-memory-graph export database -o - | \
  gzip | \
  aws s3 cp - s3://backups/llm-memory-$(date +%Y%m%d).json.gz

# Query and analyze
llm-memory-graph query -t response -f json | \
  jq '[.nodes[].data.token_usage.total_tokens] | add'
```

## Configuration

### Environment Variables

```bash
export LLM_MEMORY_GRAPH_DB_PATH=/var/lib/llm-memory-graph
export LLM_MEMORY_GRAPH_LOG_LEVEL=debug
export LLM_MEMORY_GRAPH_DEFAULT_FORMAT=json
```

### Config File

Create `~/.llm-memory-graph/config.toml`:

```toml
db_path = "/var/lib/llm-memory-graph"
default_format = "table"
log_level = "info"

[server]
host = "0.0.0.0"
port = 50051
enable_metrics = true

[export]
default_format = "json"
compression = true
```

## See Also

- [CLI README](README.md)
- [Examples](examples.md)
- [API Documentation](../API.md)
