# LLM Memory Graph CLI Reference

The `llm-memory-graph` command-line tool provides comprehensive management and query capabilities for the LLM Memory Graph database.

## Table of Contents

- [Installation](#installation)
- [Global Options](#global-options)
- [Commands Overview](#commands-overview)
- [Command Reference](#command-reference)
- [Examples](#examples)

## Installation

### From Source

```bash
cargo install --path crates/llm-memory-graph-cli
```

### Binary Release

Download the latest release from the [releases page](https://github.com/globalbusinessadvisors/llm-memory-graph/releases).

## Global Options

These options apply to all commands:

- `-d, --db-path <DB_PATH>` - Path to the database directory (default: `./data`)
- `-f, --format <FORMAT>` - Output format: `text`, `json`, `yaml`, or `table` (default: `text`)
- `-h, --help` - Print help information
- `-V, --version` - Print version information

## Commands Overview

| Command | Description |
|---------|-------------|
| `stats` | Show database statistics |
| `session` | Session management commands |
| `node` | Node operations |
| `query` | Advanced query with filters |
| `export` | Export operations |
| `import` | Import operations |
| `template` | Template management |
| `agent` | Agent management |
| `server` | Server management |
| `flush` | Flush database to disk |
| `verify` | Verify database integrity |

## Command Reference

### stats

Show comprehensive database statistics including node counts, edge counts, and session information.

```bash
llm-memory-graph stats
```

**Options:**
- None

**Output:**
- Total nodes and edges
- Active and total sessions
- Node type breakdown
- Database size information

### session

Manage sessions in the memory graph.

#### session get

Get detailed information about a specific session.

```bash
llm-memory-graph session get <SESSION_ID>
```

**Arguments:**
- `<SESSION_ID>` - Session UUID

**Options:**
- None

**Output:**
- Session metadata
- Creation and update timestamps
- Active status
- Associated nodes

### node

Operations on individual nodes.

#### node get

Get detailed information about a specific node.

```bash
llm-memory-graph node get <NODE_ID>
```

**Arguments:**
- `<NODE_ID>` - Node UUID

**Options:**
- None

**Output:**
- Node type and ID
- Creation timestamp
- Node-specific data (content, metadata, etc.)
- Connected edges

### query

Advanced query with multiple filter options.

```bash
llm-memory-graph query [OPTIONS]
```

**Options:**
- `-s, --session <SESSION_ID>` - Filter by session ID (UUID format)
- `-t, --node-type <TYPE>` - Filter by node type (`prompt`, `response`, `agent`, `template`, `tool`)
- `-a, --after <TIMESTAMP>` - Filter by creation time after this timestamp (RFC3339 format)
- `-b, --before <TIMESTAMP>` - Filter by creation time before this timestamp (RFC3339 format)
- `-l, --limit <NUMBER>` - Limit number of results

**Examples:**

```bash
# Get all prompts in a session
llm-memory-graph query -s <session-id> -t prompt

# Get recent responses
llm-memory-graph query -t response -a 2024-01-01T00:00:00Z

# Get latest 10 nodes
llm-memory-graph query -l 10
```

### export

Export data from the memory graph.

#### export session

Export a complete session with all associated nodes.

```bash
llm-memory-graph export session <SESSION_ID> -o <OUTPUT_FILE> [--export-format <FORMAT>]
```

**Arguments:**
- `<SESSION_ID>` - Session UUID to export

**Options:**
- `-o, --output <FILE>` - Output file path (required)
- `--export-format <FORMAT>` - Export format: `json` or `msgpack` (default: `json`)

#### export database

Export the entire database.

```bash
llm-memory-graph export database -o <OUTPUT_FILE> [--export-format <FORMAT>]
```

**Options:**
- `-o, --output <FILE>` - Output file path (required)
- `--export-format <FORMAT>` - Export format: `json` or `msgpack` (default: `json`)

### import

Import data into the memory graph.

```bash
llm-memory-graph import -i <INPUT_FILE> [--import-format <FORMAT>] [--dry-run]
```

**Options:**
- `-i, --input <FILE>` - Input file path (required)
- `--import-format <FORMAT>` - Import format: `json` or `msgpack` (default: `json`)
- `--dry-run` - Validate without importing

**Examples:**

```bash
# Import from JSON file
llm-memory-graph import -i backup.json

# Validate import without applying
llm-memory-graph import -i backup.json --dry-run

# Import MessagePack format
llm-memory-graph import -i backup.msgpack --import-format msgpack
```

### template

Manage prompt templates.

#### template list

List all templates in the database.

```bash
llm-memory-graph template list
```

#### template get

Get details of a specific template.

```bash
llm-memory-graph template get <TEMPLATE_ID>
```

**Arguments:**
- `<TEMPLATE_ID>` - Template UUID

#### template create

Create a new template.

```bash
llm-memory-graph template create --name <NAME> --text <TEMPLATE_TEXT>
```

**Options:**
- `--name <NAME>` - Template name (required)
- `--text <TEXT>` - Template text with variables (required)
- `--variables <VARS>` - Variable specifications (JSON format)

### agent

Manage AI agents.

#### agent list

List all agents in the database.

```bash
llm-memory-graph agent list
```

#### agent get

Get details of a specific agent.

```bash
llm-memory-graph agent get <AGENT_ID>
```

**Arguments:**
- `<AGENT_ID>` - Agent UUID

#### agent create

Create a new agent.

```bash
llm-memory-graph agent create --name <NAME> --role <ROLE>
```

**Options:**
- `--name <NAME>` - Agent name (required)
- `--role <ROLE>` - Agent role (required)
- `--capabilities <CAPS>` - Agent capabilities (comma-separated)

### server

Manage the gRPC server.

#### server start

Start the gRPC server.

```bash
llm-memory-graph server start [OPTIONS]
```

**Options:**
- `--host <HOST>` - Server host (default: `0.0.0.0`)
- `--port <PORT>` - Server port (default: `50051`)
- `--tls-cert <FILE>` - TLS certificate file
- `--tls-key <FILE>` - TLS private key file

#### server stop

Stop a running gRPC server.

```bash
llm-memory-graph server stop [--host <HOST>] [--port <PORT>]
```

**Options:**
- `--host <HOST>` - Server host (default: `localhost`)
- `--port <PORT>` - Server port (default: `50051`)

### flush

Force flush the database to disk.

```bash
llm-memory-graph flush
```

**Options:**
- None

**Description:**
Ensures all pending writes are persisted to disk. Useful before shutdown or backup operations.

### verify

Verify database integrity.

```bash
llm-memory-graph verify
```

**Options:**
- None

**Description:**
Checks database consistency, validates references, and reports any issues.

## Examples

### Basic Operations

```bash
# View database statistics
llm-memory-graph stats

# Get session details
llm-memory-graph session get 550e8400-e29b-41d4-a716-446655440000

# Query all prompts
llm-memory-graph query -t prompt -f table
```

### Export and Import

```bash
# Export a session
llm-memory-graph export session <session-id> -o session-backup.json

# Export entire database
llm-memory-graph export database -o full-backup.json

# Import with validation
llm-memory-graph import -i backup.json --dry-run
llm-memory-graph import -i backup.json
```

### Template Management

```bash
# List all templates
llm-memory-graph template list -f table

# Create a new template
llm-memory-graph template create \
  --name "greeting" \
  --text "Hello {{name}}, welcome to {{location}}!"

# Get template details
llm-memory-graph template get <template-id>
```

### Advanced Queries

```bash
# Get all responses after a specific date
llm-memory-graph query \
  -t response \
  -a 2024-01-01T00:00:00Z \
  -f json

# Get prompts in a session with limit
llm-memory-graph query \
  -s <session-id> \
  -t prompt \
  -l 10 \
  -f table

# Get nodes in a time range
llm-memory-graph query \
  -a 2024-01-01T00:00:00Z \
  -b 2024-02-01T00:00:00Z \
  -f yaml
```

### Server Management

```bash
# Start server with TLS
llm-memory-graph server start \
  --host 0.0.0.0 \
  --port 50051 \
  --tls-cert /path/to/cert.pem \
  --tls-key /path/to/key.pem

# Stop server
llm-memory-graph server stop --port 50051
```

### Maintenance

```bash
# Flush database to disk
llm-memory-graph flush

# Verify database integrity
llm-memory-graph verify

# View statistics in JSON format
llm-memory-graph stats -f json > stats.json
```

## Output Formats

The CLI supports multiple output formats via the `-f, --format` flag:

### text (default)

Human-readable text output with clear formatting.

### json

Machine-readable JSON output, suitable for scripting and automation.

```bash
llm-memory-graph stats -f json | jq '.total_nodes'
```

### yaml

YAML output for configuration and readability.

```bash
llm-memory-graph query -t prompt -f yaml
```

### table

Tabular output for easy reading of list data.

```bash
llm-memory-graph template list -f table
```

## Exit Codes

- `0` - Success
- `1` - General error
- `2` - Invalid arguments
- `3` - Database error
- `4` - Network error

## Environment Variables

- `LLM_MEMORY_GRAPH_DB_PATH` - Default database path
- `LLM_MEMORY_GRAPH_LOG_LEVEL` - Log level (`debug`, `info`, `warn`, `error`)

## Configuration File

The CLI can read from a configuration file at `~/.llm-memory-graph/config.toml`:

```toml
db_path = "/var/lib/llm-memory-graph"
default_format = "table"
log_level = "info"

[server]
host = "0.0.0.0"
port = 50051

[export]
default_format = "json"
```

## Troubleshooting

### Database Locked

If you see a "database locked" error, ensure no other processes are accessing the database:

```bash
# Flush and close cleanly
llm-memory-graph flush
```

### Permission Denied

Ensure the database directory has proper permissions:

```bash
chmod 755 /path/to/data
```

### Invalid UUID Format

UUIDs must be in the standard format: `550e8400-e29b-41d4-a716-446655440000`

## See Also

- [API Documentation](../API.md)
- [User Guide](../guides/quickstart.md)
- [Examples](../EXAMPLES.md)
