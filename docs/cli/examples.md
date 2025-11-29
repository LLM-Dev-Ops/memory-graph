# CLI Usage Examples

Practical examples for common LLM Memory Graph CLI tasks.

## Table of Contents

- [Getting Started](#getting-started)
- [Database Inspection](#database-inspection)
- [Session Management](#session-management)
- [Querying Data](#querying-data)
- [Export and Backup](#export-and-backup)
- [Import and Restore](#import-and-restore)
- [Template Management](#template-management)
- [Agent Management](#agent-management)
- [Server Operations](#server-operations)
- [Maintenance](#maintenance)
- [Automation and Scripting](#automation-and-scripting)

## Getting Started

### View Help

```bash
# General help
llm-memory-graph --help

# Command-specific help
llm-memory-graph query --help
llm-memory-graph export --help
```

### Check Version

```bash
llm-memory-graph --version
```

### Specify Database Location

```bash
# Use custom database path
llm-memory-graph --db-path /var/lib/llm-memory-graph stats

# Set environment variable
export LLM_MEMORY_GRAPH_DB_PATH=/var/lib/llm-memory-graph
llm-memory-graph stats
```

## Database Inspection

### View Statistics

```bash
# Basic statistics
llm-memory-graph stats

# JSON format for parsing
llm-memory-graph stats -f json

# Pretty table format
llm-memory-graph stats -f table
```

Example output:

```
Database Statistics
===================
Total Nodes: 1,523
Total Edges: 2,047
Total Sessions: 45
Active Sessions: 3

Node Types:
  Prompts: 512
  Responses: 498
  Tool Invocations: 413
  Templates: 67
  Agents: 33
```

### Verify Database Health

```bash
# Check database integrity
llm-memory-graph verify

# With custom database path
llm-memory-graph --db-path /data/llm verify
```

## Session Management

### Get Session Details

```bash
# View session information
llm-memory-graph session get 550e8400-e29b-41d4-a716-446655440000

# JSON output
llm-memory-graph session get 550e8400-e29b-41d4-a716-446655440000 -f json

# YAML output
llm-memory-graph session get 550e8400-e29b-41d4-a716-446655440000 -f yaml
```

### List All Sessions

```bash
# Query all session nodes
llm-memory-graph query -t session -f table

# Get session count
llm-memory-graph stats -f json | jq '.total_sessions'
```

## Querying Data

### Query by Node Type

```bash
# Get all prompts
llm-memory-graph query -t prompt

# Get all responses
llm-memory-graph query -t response

# Get all tool invocations
llm-memory-graph query -t tool_invocation

# Get all templates
llm-memory-graph query -t template

# Get all agents
llm-memory-graph query -t agent
```

### Query by Session

```bash
# Get all nodes in a session
llm-memory-graph query -s 550e8400-e29b-41d4-a716-446655440000

# Get prompts in a session
llm-memory-graph query \
  -s 550e8400-e29b-41d4-a716-446655440000 \
  -t prompt

# Get responses in a session
llm-memory-graph query \
  -s 550e8400-e29b-41d4-a716-446655440000 \
  -t response
```

### Query by Time Range

```bash
# Nodes created after a specific time
llm-memory-graph query -a 2024-01-01T00:00:00Z

# Nodes created before a specific time
llm-memory-graph query -b 2024-02-01T00:00:00Z

# Nodes in a time range
llm-memory-graph query \
  -a 2024-01-01T00:00:00Z \
  -b 2024-02-01T00:00:00Z

# Last 24 hours
llm-memory-graph query -a $(date -u -d '24 hours ago' +%Y-%m-%dT%H:%M:%SZ)
```

### Query with Limits

```bash
# Get latest 10 nodes
llm-memory-graph query -l 10

# Get latest 50 prompts
llm-memory-graph query -t prompt -l 50

# Get latest 20 responses in a session
llm-memory-graph query \
  -s 550e8400-e29b-41d4-a716-446655440000 \
  -t response \
  -l 20
```

### Complex Queries

```bash
# Recent prompts in a session
llm-memory-graph query \
  -s 550e8400-e29b-41d4-a716-446655440000 \
  -t prompt \
  -a 2024-01-15T00:00:00Z \
  -l 30 \
  -f table

# All responses from January 2024
llm-memory-graph query \
  -t response \
  -a 2024-01-01T00:00:00Z \
  -b 2024-02-01T00:00:00Z \
  -f json

# Latest 100 tool invocations
llm-memory-graph query \
  -t tool_invocation \
  -l 100 \
  -f yaml
```

## Export and Backup

### Export a Session

```bash
# Export to JSON
llm-memory-graph export session \
  550e8400-e29b-41d4-a716-446655440000 \
  -o session-backup.json

# Export to MessagePack (smaller size)
llm-memory-graph export session \
  550e8400-e29b-41d4-a716-446655440000 \
  -o session-backup.msgpack \
  --export-format msgpack

# Export with timestamp
llm-memory-graph export session \
  550e8400-e29b-41d4-a716-446655440000 \
  -o "session-$(date +%Y%m%d-%H%M%S).json"
```

### Export Full Database

```bash
# Export to JSON
llm-memory-graph export database -o full-backup.json

# Export to MessagePack
llm-memory-graph export database \
  -o full-backup.msgpack \
  --export-format msgpack

# Daily backup with date
llm-memory-graph export database \
  -o "backup-$(date +%Y%m%d).json"

# Compressed backup
llm-memory-graph export database -o - | gzip > backup.json.gz
```

### Automated Backups

```bash
#!/bin/bash
# daily-backup.sh

BACKUP_DIR="/backups/llm-memory-graph"
DATE=$(date +%Y%m%d)

# Create backup
llm-memory-graph export database -o "$BACKUP_DIR/backup-$DATE.json"

# Compress
gzip "$BACKUP_DIR/backup-$DATE.json"

# Remove backups older than 30 days
find "$BACKUP_DIR" -name "backup-*.json.gz" -mtime +30 -delete

echo "Backup completed: backup-$DATE.json.gz"
```

## Import and Restore

### Import Data

```bash
# Import from JSON
llm-memory-graph import -i backup.json

# Import from MessagePack
llm-memory-graph import \
  -i backup.msgpack \
  --import-format msgpack

# Dry run (validation only)
llm-memory-graph import -i backup.json --dry-run
```

### Restore from Backup

```bash
# Validate backup first
llm-memory-graph import -i backup.json --dry-run

# Restore if valid
if [ $? -eq 0 ]; then
  llm-memory-graph import -i backup.json
  echo "Restore completed"
else
  echo "Backup validation failed"
  exit 1
fi
```

### Migration Between Databases

```bash
# Export from source
llm-memory-graph --db-path /old/data export database -o migration.json

# Import to destination
llm-memory-graph --db-path /new/data import -i migration.json
```

## Template Management

### List Templates

```bash
# List all templates
llm-memory-graph template list

# Table format
llm-memory-graph template list -f table

# JSON format
llm-memory-graph template list -f json
```

### View Template

```bash
# Get template details
llm-memory-graph template get a1b2c3d4-e5f6-7890-abcd-ef1234567890

# JSON output
llm-memory-graph template get \
  a1b2c3d4-e5f6-7890-abcd-ef1234567890 \
  -f json
```

### Create Template

```bash
# Simple template
llm-memory-graph template create \
  --name "greeting" \
  --text "Hello {{name}}!"

# Template with multiple variables
llm-memory-graph template create \
  --name "email" \
  --text "Dear {{recipient}}, {{message}} Best regards, {{sender}}"

# Template with variable specs
llm-memory-graph template create \
  --name "code_review" \
  --text "Review this {{language}} code: {{code}}" \
  --variables '[
    {"name":"language","required":true},
    {"name":"code","required":true}
  ]'
```

## Agent Management

### List Agents

```bash
# List all agents
llm-memory-graph agent list

# Table format
llm-memory-graph agent list -f table
```

### View Agent

```bash
# Get agent details
llm-memory-graph agent get c3d4e5f6-a7b8-9012-cdef-123456789012

# JSON output
llm-memory-graph agent get \
  c3d4e5f6-a7b8-9012-cdef-123456789012 \
  -f json
```

### Create Agent

```bash
# Basic agent
llm-memory-graph agent create \
  --name "Assistant" \
  --role "helper"

# Agent with capabilities
llm-memory-graph agent create \
  --name "CodeReviewer" \
  --role "reviewer" \
  --capabilities "python,javascript,rust,code-analysis"

# Specialized agent
llm-memory-graph agent create \
  --name "DataAnalyst" \
  --role "analyst" \
  --capabilities "sql,python,statistics,visualization"
```

## Server Operations

### Start Server

```bash
# Start with defaults
llm-memory-graph server start

# Custom host and port
llm-memory-graph server start --host 127.0.0.1 --port 8080

# Start with TLS
llm-memory-graph server start \
  --host 0.0.0.0 \
  --port 50051 \
  --tls-cert /etc/certs/server.crt \
  --tls-key /etc/certs/server.key

# Start in background
nohup llm-memory-graph server start > server.log 2>&1 &
```

### Stop Server

```bash
# Stop default server
llm-memory-graph server stop

# Stop custom port
llm-memory-graph server stop --port 8080
```

### Server with Systemd

Create `/etc/systemd/system/llm-memory-graph.service`:

```ini
[Unit]
Description=LLM Memory Graph Server
After=network.target

[Service]
Type=simple
User=llm-memory
WorkingDirectory=/var/lib/llm-memory-graph
ExecStart=/usr/local/bin/llm-memory-graph server start --host 0.0.0.0 --port 50051
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

Manage service:

```bash
# Start service
sudo systemctl start llm-memory-graph

# Enable on boot
sudo systemctl enable llm-memory-graph

# Check status
sudo systemctl status llm-memory-graph

# View logs
sudo journalctl -u llm-memory-graph -f
```

## Maintenance

### Flush Database

```bash
# Flush to disk
llm-memory-graph flush

# Before backup
llm-memory-graph flush && \
  llm-memory-graph export database -o backup.json
```

### Verify Integrity

```bash
# Basic verification
llm-memory-graph verify

# Verify after import
llm-memory-graph import -i backup.json && \
  llm-memory-graph verify
```

### Database Cleanup

```bash
#!/bin/bash
# cleanup.sh - Database maintenance script

echo "Starting database maintenance..."

# Verify integrity
echo "Verifying database..."
if ! llm-memory-graph verify; then
  echo "Database verification failed!"
  exit 1
fi

# Create backup
echo "Creating backup..."
llm-memory-graph export database -o "backup-$(date +%Y%m%d).json"

# Flush to disk
echo "Flushing to disk..."
llm-memory-graph flush

# Verify again
echo "Final verification..."
llm-memory-graph verify

echo "Maintenance complete!"
```

## Automation and Scripting

### Node Count by Type

```bash
#!/bin/bash
# count-nodes.sh

llm-memory-graph stats -f json | jq -r '
  .node_types | to_entries[] |
  "\(.key): \(.value)"
'
```

### Export All Sessions

```bash
#!/bin/bash
# export-all-sessions.sh

OUTPUT_DIR="./session-exports"
mkdir -p "$OUTPUT_DIR"

# Get all sessions (assuming query returns session IDs)
llm-memory-graph query -t session -f json | \
  jq -r '.nodes[].id' | \
  while read -r session_id; do
    echo "Exporting session: $session_id"
    llm-memory-graph export session "$session_id" \
      -o "$OUTPUT_DIR/session-$session_id.json"
  done

echo "Export complete. Files in $OUTPUT_DIR"
```

### Monitor Database Growth

```bash
#!/bin/bash
# monitor-growth.sh

while true; do
  DATE=$(date +"%Y-%m-%d %H:%M:%S")
  STATS=$(llm-memory-graph stats -f json)
  NODES=$(echo "$STATS" | jq '.total_nodes')
  EDGES=$(echo "$STATS" | jq '.total_edges')

  echo "$DATE - Nodes: $NODES, Edges: $EDGES" >> growth.log

  sleep 3600  # Check every hour
done
```

### Batch Query Processing

```bash
#!/bin/bash
# process-prompts.sh

# Get all prompts from last week
llm-memory-graph query \
  -t prompt \
  -a $(date -u -d '7 days ago' +%Y-%m-%dT%H:%M:%SZ) \
  -f json | \
  jq -r '.nodes[] | {
    id: .id,
    content: .data.content,
    model: .data.metadata.model
  }' | \
  while read -r prompt; do
    # Process each prompt
    echo "$prompt" >> prompts-analysis.json
  done
```

### Database Health Check

```bash
#!/bin/bash
# health-check.sh

check_health() {
  # Verify database
  if ! llm-memory-graph verify > /dev/null 2>&1; then
    echo "CRITICAL: Database verification failed"
    return 1
  fi

  # Check node count
  NODES=$(llm-memory-graph stats -f json | jq '.total_nodes')
  if [ "$NODES" -eq 0 ]; then
    echo "WARNING: No nodes in database"
    return 1
  fi

  echo "OK: Database healthy ($NODES nodes)"
  return 0
}

# Run check
if check_health; then
  exit 0
else
  # Send alert
  echo "Database health check failed!" | mail -s "LLM Memory Graph Alert" admin@example.com
  exit 1
fi
```

### Incremental Backup

```bash
#!/bin/bash
# incremental-backup.sh

BACKUP_DIR="/backups/llm-memory-graph"
LAST_BACKUP="$BACKUP_DIR/last-backup-time.txt"

# Get last backup timestamp
if [ -f "$LAST_BACKUP" ]; then
  LAST_TIME=$(cat "$LAST_BACKUP")
else
  LAST_TIME="2000-01-01T00:00:00Z"
fi

# Export nodes created since last backup
CURRENT_TIME=$(date -u +%Y-%m-%dT%H:%M:%SZ)
OUTPUT_FILE="$BACKUP_DIR/incremental-$(date +%Y%m%d-%H%M%S).json"

llm-memory-graph query \
  -a "$LAST_TIME" \
  -f json > "$OUTPUT_FILE"

# Update last backup time
echo "$CURRENT_TIME" > "$LAST_BACKUP"

echo "Incremental backup saved to $OUTPUT_FILE"
```

### Report Generation

```bash
#!/bin/bash
# generate-report.sh

REPORT_FILE="report-$(date +%Y%m%d).txt"

{
  echo "LLM Memory Graph Report"
  echo "======================"
  echo "Generated: $(date)"
  echo ""

  echo "Database Statistics:"
  llm-memory-graph stats
  echo ""

  echo "Recent Activity (Last 24 Hours):"
  llm-memory-graph query \
    -a $(date -u -d '24 hours ago' +%Y-%m-%dT%H:%M:%SZ) \
    -f table
  echo ""

  echo "Database Verification:"
  llm-memory-graph verify

} > "$REPORT_FILE"

echo "Report generated: $REPORT_FILE"
```

## Tips and Best Practices

### Performance Tips

```bash
# Use limits for large queries
llm-memory-graph query -t prompt -l 1000

# Use MessagePack for large exports
llm-memory-graph export database \
  -o backup.msgpack \
  --export-format msgpack

# Flush before intensive operations
llm-memory-graph flush
```

### Data Integrity

```bash
# Always verify after import
llm-memory-graph import -i backup.json && \
  llm-memory-graph verify

# Dry run before importing
llm-memory-graph import -i backup.json --dry-run
```

### Output Formatting

```bash
# JSON for scripting
llm-memory-graph stats -f json | jq '.total_nodes'

# Table for readability
llm-memory-graph template list -f table

# YAML for configuration
llm-memory-graph query -t agent -f yaml > agents.yaml
```

## See Also

- [CLI README](README.md)
- [Commands Reference](commands.md)
- [API Documentation](../API.md)
- [User Guide](../guides/quickstart.md)
