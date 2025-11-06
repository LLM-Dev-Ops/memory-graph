# LLM-Memory-Graph Integration Runbooks

## Operational Procedures for Production Integrations

This document provides step-by-step operational procedures for common scenarios, incident response, and troubleshooting.

---

## TABLE OF CONTENTS

1. [Observatory Integration Operations](#1-observatory-integration-operations)
2. [Registry Integration Operations](#2-registry-integration-operations)
3. [Vault Integration Operations](#3-vault-integration-operations)
4. [Incident Response Procedures](#4-incident-response-procedures)
5. [Performance Tuning](#5-performance-tuning)
6. [Security Operations](#6-security-operations)
7. [Disaster Recovery](#7-disaster-recovery)

---

## 1. OBSERVATORY INTEGRATION OPERATIONS

### 1.1 Kafka Consumer Lag Investigation

**Symptoms:**
- Alert: `KafkaConsumerLag > 10000 messages`
- Delayed event processing
- Memory pressure

**Investigation Steps:**

```bash
# 1. Check consumer group lag
kafka-consumer-groups --bootstrap-server kafka:9092 \
  --group memory-graph-consumers \
  --describe

# 2. Check partition distribution
kubectl get pods -n llm-platform -l app=llm-memory-graph

# 3. Check consumer metrics
curl http://llm-memory-graph:9090/metrics | grep kafka_lag

# 4. Check graph database performance
curl http://neo4j:7474/db/system/tx/commit \
  -H "Authorization: Basic $(echo -n user:pass | base64)" \
  -d '{"statements":[{"statement":"CALL dbms.queryJmx(\"org.neo4j:*\")"}]}'
```

**Resolution:**

**Option A: Scale Consumers**
```bash
# Increase replicas
kubectl scale deployment llm-memory-graph -n llm-platform --replicas=6

# Verify partition rebalancing
kafka-consumer-groups --bootstrap-server kafka:9092 \
  --group memory-graph-consumers \
  --describe
```

**Option B: Increase Buffer Size**
```yaml
# Update ConfigMap
kubectl edit configmap llm-memory-graph-config -n llm-platform

# Change:
# buffer_size_mb: 512  # Increased from 256

# Restart pods to apply
kubectl rollout restart deployment llm-memory-graph -n llm-platform
```

**Option C: Optimize Graph Writes**
```cypher
// Add indices to speed up writes
CREATE INDEX invocation_id IF NOT EXISTS FOR (i:Invocation) ON (i.id);
CREATE INDEX model_id IF NOT EXISTS FOR (m:Model) ON (m.id);
CREATE INDEX timestamp IF NOT EXISTS FOR (i:Invocation) ON (i.timestamp);
```

**Prevention:**
- Monitor lag continuously
- Set up autoscaling based on lag metric
- Pre-warm connection pools during traffic spikes

---

### 1.2 Circuit Breaker Activation

**Symptoms:**
- Alert: `CircuitBreakerOpen`
- Events accumulating in buffer
- Graph write failures

**Investigation:**

```bash
# 1. Check circuit breaker status
curl http://llm-memory-graph:8080/health/circuit-breaker

# 2. Check graph database health
curl http://neo4j:7474/db/neo4j/cluster/available

# 3. Check network connectivity
kubectl exec -it llm-memory-graph-pod -n llm-platform -- \
  nc -zv neo4j-cluster 7687

# 4. Review error logs
kubectl logs -n llm-platform -l app=llm-memory-graph \
  --tail=100 | grep "circuit breaker"
```

**Resolution:**

**If Graph DB is healthy:**
```bash
# 1. Force circuit breaker reset (use with caution)
curl -X POST http://llm-memory-graph:8080/admin/circuit-breaker/reset

# 2. Monitor recovery
watch -n 2 "curl -s http://llm-memory-graph:9090/metrics | grep circuit_breaker_state"
```

**If Graph DB has issues:**
```bash
# 1. Check Neo4j logs
kubectl logs -n llm-platform neo4j-0 --tail=100

# 2. Check resource usage
kubectl top pod neo4j-0 -n llm-platform

# 3. Scale Neo4j if needed
kubectl scale statefulset neo4j -n llm-platform --replicas=5

# 4. Wait for circuit breaker auto-recovery (30s)
```

---

### 1.3 Event Processing Latency Spike

**Symptoms:**
- Alert: `p99_latency > 500ms`
- Slow dashboard updates
- User complaints

**Investigation:**

```bash
# 1. Check processing duration histogram
curl -s http://llm-memory-graph:9090/metrics | \
  grep event_processing_duration_seconds_bucket

# 2. Check distributed traces
curl "http://jaeger:16686/api/traces?service=llm-memory-graph&limit=20"

# 3. Profile application
kubectl exec -it llm-memory-graph-pod -n llm-platform -- \
  curl http://localhost:6060/debug/pprof/profile?seconds=30 > profile.out

# 4. Check for slow queries
# In Neo4j browser or cypher-shell:
CALL dbms.listQueries()
YIELD queryId, query, elapsedTimeMillis
WHERE elapsedTimeMillis > 1000
RETURN queryId, query, elapsedTimeMillis
ORDER BY elapsedTimeMillis DESC;
```

**Resolution:**

```bash
# 1. Kill slow queries
CALL dbms.killQuery("query-123");

# 2. Optimize common query patterns
# Add composite indices
CREATE INDEX invocation_model_timestamp IF NOT EXISTS
FOR (i:Invocation) ON (i.model_id, i.timestamp);

# 3. Increase batch size for better throughput
kubectl set env deployment/llm-memory-graph \
  BATCH_SIZE=2000 -n llm-platform

# 4. Enable query caching
# In Neo4j config:
dbms.query_cache_size=1000
```

---

## 2. REGISTRY INTEGRATION OPERATIONS

### 2.1 Metadata Sync Failure

**Symptoms:**
- Alert: `RegistrySyncFailures > 10/min`
- Stale model metadata in graph
- Missing model capabilities

**Investigation:**

```bash
# 1. Check Registry API health
curl https://llm-registry:8080/health

# 2. Check GraphQL endpoint
curl -X POST https://llm-registry:8080/graphql \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"query": "{ __schema { queryType { name } } }"}'

# 3. Check CDC stream
kafka-console-consumer --bootstrap-server kafka:9092 \
  --topic registry.public.models \
  --max-messages 10

# 4. Check sync service logs
kubectl logs -n llm-platform -l app=llm-memory-graph,component=sync \
  --tail=100
```

**Resolution:**

**If Registry is unavailable:**
```bash
# 1. Check Registry pods
kubectl get pods -n llm-platform -l app=llm-registry

# 2. Check Registry database
kubectl exec -it llm-registry-db-0 -n llm-platform -- \
  psql -U registry -c "SELECT 1"

# 3. Restart Registry if needed
kubectl rollout restart deployment llm-registry -n llm-platform
```

**If CDC stream is broken:**
```bash
# 1. Check Debezium connector status
curl http://kafka-connect:8083/connectors/registry-source/status

# 2. Restart connector if needed
curl -X POST http://kafka-connect:8083/connectors/registry-source/restart

# 3. Verify messages flowing
kafka-console-consumer --bootstrap-server kafka:9092 \
  --topic registry.public.models \
  --from-beginning \
  --max-messages 5
```

**If sync logic has errors:**
```bash
# 1. Enable debug logging
kubectl set env deployment/llm-memory-graph \
  LOG_LEVEL=debug -n llm-platform

# 2. Trigger manual sync
curl -X POST http://llm-memory-graph:8080/admin/sync/trigger \
  -H "Content-Type: application/json" \
  -d '{"source": "registry", "full_sync": false}'

# 3. Check sync status
curl http://llm-memory-graph:8080/admin/sync/status
```

---

### 2.2 Schema Version Mismatch

**Symptoms:**
- Alert: `SchemaVersionMismatch`
- Parsing errors in logs
- Failed model updates

**Investigation:**

```bash
# 1. Check current schema versions
curl http://llm-registry:8080/api/v1/schemas/versions

# 2. Check Memory-Graph supported versions
curl http://llm-memory-graph:8080/admin/schemas/supported

# 3. Check for schema validation errors
kubectl logs -n llm-platform -l app=llm-memory-graph \
  | grep "schema validation"
```

**Resolution:**

**For backward-compatible changes:**
```bash
# 1. Deploy schema migration
kubectl apply -f migrations/schema-v2.0.0.yaml

# 2. Verify migration
curl http://llm-memory-graph:8080/admin/migrations/status

# 3. Update supported versions
curl -X POST http://llm-memory-graph:8080/admin/schemas/add-version \
  -d '{"version": "2.0.0"}'
```

**For breaking changes:**
```bash
# 1. Enable compatibility mode
kubectl set env deployment/llm-memory-graph \
  SCHEMA_COMPATIBILITY_MODE=true -n llm-platform

# 2. Schedule maintenance window
# 3. Coordinate with Registry team for synchronized update
# 4. Deploy new version with dual schema support
kubectl apply -f deploy/llm-memory-graph-v2.yaml

# 5. Gradually migrate data
curl -X POST http://llm-memory-graph:8080/admin/migrate/schema \
  -d '{"from_version": "1.0.0", "to_version": "2.0.0", "batch_size": 1000}'

# 6. Monitor migration progress
watch -n 5 "curl -s http://llm-memory-graph:8080/admin/migrate/progress"
```

---

### 2.3 Conflict Resolution Issues

**Symptoms:**
- Inconsistent model data between Registry and Graph
- Duplicate model nodes
- Version history gaps

**Investigation:**

```bash
# 1. Compare data between systems
# Get from Registry:
curl http://llm-registry:8080/api/v1/models/gpt-4

# Get from Graph:
curl http://llm-memory-graph:8080/api/v1/graph/models/gpt-4

# 2. Check conflict logs
kubectl logs -n llm-platform -l app=llm-memory-graph \
  | grep "conflict detected"

# 3. Query for duplicate nodes
# In Neo4j:
MATCH (m:Model)
WITH m.id as model_id, count(*) as count
WHERE count > 1
RETURN model_id, count;
```

**Resolution:**

```bash
# 1. Identify authoritative source (usually Registry)
# 2. Trigger reconciliation
curl -X POST http://llm-memory-graph:8080/admin/reconcile \
  -d '{"source": "registry", "model_id": "gpt-4", "force": true}'

# 3. Merge duplicate nodes
# In Neo4j:
MATCH (m1:Model {id: "gpt-4"}), (m2:Model {id: "gpt-4"})
WHERE id(m1) < id(m2)
WITH m1, m2
OPTIONAL MATCH (inv)-[r:USES_MODEL]->(m2)
DELETE r
CREATE (inv)-[:USES_MODEL]->(m1)
WITH m1, m2
DELETE m2;

# 4. Verify fix
curl http://llm-memory-graph:8080/api/v1/graph/models/gpt-4
```

---

## 3. VAULT INTEGRATION OPERATIONS

### 3.1 Encryption/Decryption Failures

**Symptoms:**
- Alert: `VaultOperationFailure`
- Content retrieval errors
- KMS timeout errors

**Investigation:**

```bash
# 1. Check Vault API health
curl https://llm-vault:8443/health

# 2. Check KMS connectivity
aws kms describe-key --key-id $KMS_KEY_ID

# 3. Check certificate validity
openssl s_client -connect llm-vault:8443 -showcerts

# 4. Check error logs
kubectl logs -n llm-platform -l app=llm-memory-graph \
  | grep -E "encryption|decryption|kms"
```

**Resolution:**

**If KMS is unavailable:**
```bash
# 1. Check AWS service health
aws health describe-events --filter eventTypeCategories=issue

# 2. Use alternative region KMS (if configured)
kubectl set env deployment/llm-memory-graph \
  KMS_REGION=us-west-2 -n llm-platform

# 3. Enable local key caching
kubectl set env deployment/llm-memory-graph \
  KMS_CACHE_ENABLED=true \
  KMS_CACHE_TTL=3600 -n llm-platform
```

**If certificate issues:**
```bash
# 1. Rotate certificates
kubectl create secret tls llm-memory-graph-certs \
  --cert=new-cert.pem \
  --key=new-key.pem \
  --dry-run=client -o yaml | kubectl apply -f -

# 2. Restart pods to pick up new certs
kubectl rollout restart deployment llm-memory-graph -n llm-platform

# 3. Verify connectivity
kubectl exec -it llm-memory-graph-pod -n llm-platform -- \
  curl -v --cert /etc/ssl/certs/client.pem \
       --key /etc/ssl/certs/client-key.pem \
       --cacert /etc/ssl/certs/ca.pem \
       https://llm-vault:8443/health
```

---

### 3.2 Access Control Violation

**Symptoms:**
- Alert: `VaultAccessDenied`
- 403 errors in logs
- Audit log entries for denied access

**Investigation:**

```bash
# 1. Check recent access denials
curl http://llm-vault:8443/api/v1/audit/denied \
  -H "Authorization: Bearer $ADMIN_TOKEN"

# 2. Verify user permissions
curl http://llm-vault:8443/api/v1/users/$USER_ID/permissions

# 3. Check ABAC policy evaluation
curl http://llm-vault:8443/api/v1/policies/evaluate \
  -d '{
    "user_id": "user-123",
    "resource": {"classification": "restricted", "content_id": "prompt-456"},
    "action": "read"
  }'
```

**Resolution:**

**If legitimate access needed:**
```bash
# 1. Grant temporary elevated access
curl -X POST http://llm-vault:8443/api/v1/users/$USER_ID/grant-access \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{
    "role": "compliance_officer",
    "duration_hours": 4,
    "reason": "Incident investigation INC-12345"
  }'

# 2. Document in change log
echo "$(date): Granted temporary access to $USER_ID for INC-12345" >> access-changes.log

# 3. Set reminder for revocation
```

**If policy misconfiguration:**
```bash
# 1. Review ABAC policies
curl http://llm-vault:8443/api/v1/policies | jq

# 2. Update policy
curl -X PUT http://llm-vault:8443/api/v1/policies/pii_access \
  -d '{
    "rules": [
      {
        "effect": "allow",
        "conditions": {
          "user.roles": ["compliance_officer", "admin"],
          "time.hour": {"$gte": 9, "$lte": 17},
          "network": "corporate_vpn"
        }
      }
    ]
  }'

# 3. Test policy
curl http://llm-vault:8443/api/v1/policies/test \
  -d '{"policy_id": "pii_access", "test_cases": [...]}'
```

---

### 3.3 Key Rotation Operations

**Scheduled Activity:**
- Frequency: Monthly for DEKs, Annually for KEKs
- Maintenance window required: No (zero-downtime)

**Pre-rotation Checklist:**

```bash
# 1. Backup current keys
aws kms create-alias --alias-name alias/llm-vault-backup-$(date +%Y%m%d) \
  --target-key-id $CURRENT_KEY_ID

# 2. Verify backup encryption works
aws kms encrypt --key-id alias/llm-vault-backup-$(date +%Y%m%d) \
  --plaintext "test" --query CiphertextBlob --output text

# 3. Check for in-flight operations
curl http://llm-vault:8443/api/v1/operations/in-flight

# 4. Alert operations team
```

**Rotation Procedure:**

```bash
# 1. Create new KMS key
NEW_KEY_ID=$(aws kms create-key --description "LLM Vault KEK $(date +%Y%m%d)" \
  --query KeyMetadata.KeyId --output text)

# 2. Update key alias to point to new key
aws kms update-alias --alias-name alias/llm-vault-kek \
  --target-key-id $NEW_KEY_ID

# 3. Trigger re-encryption of DEKs
curl -X POST http://llm-vault:8443/api/v1/admin/reencrypt-deks \
  -d '{
    "old_kek_id": "'$OLD_KEY_ID'",
    "new_kek_id": "'$NEW_KEY_ID'",
    "batch_size": 1000
  }'

# 4. Monitor re-encryption progress
watch -n 10 "curl -s http://llm-vault:8443/api/v1/admin/reencrypt-progress | jq"

# 5. Verify no decryption errors
kubectl logs -n llm-platform -l app=llm-vault --since=1h \
  | grep -i "decryption error" | wc -l
# Should be 0

# 6. Schedule old key deletion (30 days)
aws kms schedule-key-deletion --key-id $OLD_KEY_ID --pending-window-in-days 30

# 7. Document rotation
echo "$(date): Rotated KEK from $OLD_KEY_ID to $NEW_KEY_ID" >> key-rotation.log
```

---

## 4. INCIDENT RESPONSE PROCEDURES

### 4.1 Data Breach Response

**Severity: CRITICAL**

**Immediate Actions (0-15 minutes):**

```bash
# 1. Isolate affected systems
kubectl scale deployment llm-memory-graph -n llm-platform --replicas=0

# 2. Block external access
kubectl apply -f - <<EOF
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: isolate-memory-graph
  namespace: llm-platform
spec:
  podSelector:
    matchLabels:
      app: llm-memory-graph
  policyTypes:
  - Ingress
  - Egress
  ingress: []
  egress: []
EOF

# 3. Enable audit log retention
curl -X POST http://llm-vault:8443/api/v1/admin/audit/preserve \
  -d '{"reason": "Security incident INC-SEC-001", "duration_days": 90}'

# 4. Notify security team
curl -X POST https://slack.com/api/chat.postMessage \
  -H "Authorization: Bearer $SLACK_TOKEN" \
  -d '{"channel": "#security-incidents", "text": "CRITICAL: Potential data breach detected in llm-memory-graph"}'
```

**Investigation (15-60 minutes):**

```bash
# 1. Export audit logs
curl http://llm-vault:8443/api/v1/audit/export \
  -d '{
    "start_date": "'$(date -d '24 hours ago' -Iseconds)'",
    "end_date": "'$(date -Iseconds)'"
  }' > breach-audit-$(date +%Y%m%d-%H%M).json

# 2. Identify compromised data
cat breach-audit-*.json | jq '.[] | select(.access_granted == true) | .content_id' > compromised-content-ids.txt

# 3. Identify affected users
cat breach-audit-*.json | jq '.[] | .user_id' | sort -u > affected-users.txt

# 4. Check for privilege escalation
cat breach-audit-*.json | jq '.[] | select(.action == "role.assigned")'

# 5. Review access patterns
cat breach-audit-*.json | jq 'group_by(.user_id) | map({user: .[0].user_id, count: length}) | sort_by(.count) | reverse'
```

**Containment (1-4 hours):**

```bash
# 1. Revoke all active sessions
curl -X POST http://llm-memory-graph:8080/admin/sessions/revoke-all

# 2. Rotate all credentials
kubectl delete secret kafka-credentials registry-credentials vault-credentials -n llm-platform
# Recreate with new credentials

# 3. Force password reset for affected users
while read user_id; do
  curl -X POST http://auth-service:8080/api/v1/users/$user_id/force-password-reset
done < affected-users.txt

# 4. Trigger emergency key rotation
curl -X POST http://llm-vault:8443/api/v1/admin/emergency-key-rotation

# 5. Enable enhanced monitoring
kubectl set env deployment/llm-memory-graph \
  AUDIT_LEVEL=verbose \
  ANOMALY_DETECTION=enabled \
  -n llm-platform
```

**Recovery (4-24 hours):**

```bash
# 1. Verify systems clean
# Run security scans, verify no backdoors

# 2. Restore service with enhanced security
kubectl apply -f deploy/llm-memory-graph-hardened.yaml

# 3. Implement additional controls
# - MFA required for all access
# - IP allowlisting
# - Rate limiting tightened

# 4. Notify affected parties
# Follow legal and compliance requirements

# 5. Post-incident review
# Schedule within 48 hours
```

---

### 4.2 System Outage (Total Failure)

**Severity: HIGH**

**Diagnosis:**

```bash
# 1. Check all components
kubectl get pods -n llm-platform

# 2. Check dependencies
kubectl get pods -n llm-platform -l component=dependency

# 3. Check cluster health
kubectl get nodes
kubectl top nodes

# 4. Check recent changes
kubectl rollout history deployment/llm-memory-graph -n llm-platform
```

**Recovery:**

```bash
# 1. Rollback to last known good version
kubectl rollout undo deployment/llm-memory-graph -n llm-platform

# 2. Check database connectivity
kubectl run -it --rm debug --image=busybox --restart=Never -- \
  nc -zv neo4j-cluster 7687

# 3. Verify Kafka connectivity
kubectl run -it --rm kafka-test --image=confluentinc/cp-kafka --restart=Never -- \
  kafka-broker-api-versions --bootstrap-server kafka:9092

# 4. Check for disk space issues
kubectl exec -it neo4j-0 -n llm-platform -- df -h

# 5. Restart in order of dependency
kubectl rollout restart deployment llm-memory-graph -n llm-platform

# 6. Monitor recovery
kubectl logs -f -n llm-platform -l app=llm-memory-graph
```

---

## 5. PERFORMANCE TUNING

### 5.1 Graph Database Optimization

**Query Performance:**

```cypher
// 1. Identify slow queries
CALL dbms.listQueries()
YIELD queryId, query, elapsedTimeMillis, allocatedBytes
WHERE elapsedTimeMillis > 1000
RETURN queryId, query, elapsedTimeMillis, allocatedBytes
ORDER BY elapsedTimeMillis DESC;

// 2. Analyze query plan
PROFILE
MATCH (inv:Invocation)-[:USES_MODEL]->(m:Model {id: 'gpt-4'})
WHERE inv.timestamp > datetime() - duration({days: 7})
RETURN inv, m;

// 3. Add strategic indices
CREATE INDEX invocation_timestamp IF NOT EXISTS
FOR (i:Invocation) ON (i.timestamp);

CREATE INDEX model_provider IF NOT EXISTS
FOR (m:Model) ON (m.provider);

CREATE CONSTRAINT invocation_id IF NOT EXISTS
FOR (i:Invocation) REQUIRE i.id IS UNIQUE;

// 4. Monitor index usage
CALL db.indexes()
YIELD name, labelsOrTypes, properties, state, populationPercent
RETURN name, labelsOrTypes, properties, state, populationPercent;
```

**Memory Tuning:**

```bash
# Edit Neo4j config
kubectl edit configmap neo4j-config -n llm-platform

# Adjust these settings:
# dbms.memory.heap.initial_size=4G
# dbms.memory.heap.max_size=8G
# dbms.memory.pagecache.size=4G

# Restart Neo4j
kubectl rollout restart statefulset neo4j -n llm-platform
```

---

### 5.2 Kafka Consumer Tuning

```bash
# Optimize consumer configuration
kubectl edit configmap llm-memory-graph-config -n llm-platform

# Adjust:
# fetch.min.bytes: 1048576  # 1MB
# fetch.max.wait.ms: 500
# max.poll.records: 5000
# max.partition.fetch.bytes: 10485760  # 10MB

# Enable compression
# compression.type: snappy

# Restart to apply
kubectl rollout restart deployment llm-memory-graph -n llm-platform
```

---

## 6. SECURITY OPERATIONS

### 6.1 Certificate Renewal

**Pre-renewal:**

```bash
# 1. Check expiration dates
openssl x509 -in current-cert.pem -noout -enddate

# 2. Generate new certificates
# Use your PKI process or cert-manager

# 3. Test new certificates
openssl verify -CAfile ca.pem new-cert.pem
```

**Renewal:**

```bash
# 1. Create new secret
kubectl create secret tls llm-memory-graph-certs-new \
  --cert=new-cert.pem \
  --key=new-key.pem \
  -n llm-platform

# 2. Update deployment to use new secret
kubectl set volume deployment/llm-memory-graph \
  --add --name=certs --type=secret \
  --secret-name=llm-memory-graph-certs-new \
  --mount-path=/etc/ssl/certs-new \
  -n llm-platform

# 3. Rolling update pods
kubectl rollout status deployment/llm-memory-graph -n llm-platform

# 4. Verify
kubectl exec -it llm-memory-graph-pod -n llm-platform -- \
  openssl s_client -connect llm-vault:8443 -cert /etc/ssl/certs-new/tls.crt

# 5. Remove old secret
kubectl delete secret llm-memory-graph-certs -n llm-platform
```

---

### 6.2 Audit Log Review

**Daily Review:**

```bash
# 1. Check for anomalies
curl http://llm-vault:8443/api/v1/audit/anomalies \
  -d '{"lookback_hours": 24}' | jq

# 2. Review failed access attempts
curl http://llm-vault:8443/api/v1/audit/failed-access \
  -d '{"lookback_hours": 24}' | jq

# 3. Check for privilege escalation
curl http://llm-vault:8443/api/v1/audit/privilege-changes \
  -d '{"lookback_hours": 24}' | jq

# 4. Export for SIEM
curl http://llm-vault:8443/api/v1/audit/export \
  -d '{
    "start_date": "'$(date -d '1 day ago' -Iseconds)'",
    "format": "siem"
  }' | gzip > audit-$(date +%Y%m%d).json.gz
```

---

## 7. DISASTER RECOVERY

### 7.1 Full System Recovery

**Prerequisites:**
- Recent backups available
- DR environment provisioned
- Runbook tested quarterly

**Recovery Procedure:**

```bash
# 1. Provision DR cluster
kubectl config use-context dr-cluster

# 2. Restore Neo4j from backup
kubectl apply -f neo4j-restore-job.yaml

# Wait for completion
kubectl wait --for=condition=complete job/neo4j-restore --timeout=3600s

# 3. Restore Kafka topics
kafka-topics --bootstrap-server dr-kafka:9092 --create \
  --topic llm-telemetry-events \
  --partitions 12 --replication-factor 3

# Restore from backup
kafka-mirror-maker --consumer.config consumer.properties \
  --producer.config producer.properties \
  --whitelist "llm-.*"

# 4. Deploy applications
kubectl apply -k deploy/dr/

# 5. Verify connectivity
./scripts/health-check.sh

# 6. Update DNS to point to DR
# Follow DNS failover procedure

# 7. Monitor for 24 hours
kubectl logs -f -n llm-platform -l app=llm-memory-graph
```

**RTO (Recovery Time Objective):** 15 minutes
**RPO (Recovery Point Objective):** 5 minutes

---

## APPENDIX A: MONITORING DASHBOARDS

### Grafana Dashboard JSON

```json
{
  "dashboard": {
    "title": "LLM Memory Graph - Integration Health",
    "panels": [
      {
        "title": "Kafka Consumer Lag",
        "targets": [
          {
            "expr": "kafka_consumer_lag{job='llm-memory-graph'}",
            "legendFormat": "{{partition}}"
          }
        ]
      },
      {
        "title": "Event Processing Latency",
        "targets": [
          {
            "expr": "histogram_quantile(0.99, rate(event_processing_duration_seconds_bucket[5m]))",
            "legendFormat": "p99"
          }
        ]
      },
      {
        "title": "Circuit Breaker Status",
        "targets": [
          {
            "expr": "circuit_breaker_state",
            "legendFormat": "{{service}}"
          }
        ]
      },
      {
        "title": "Vault Operations",
        "targets": [
          {
            "expr": "rate(vault_operations_total[5m])",
            "legendFormat": "{{operation}} - {{status}}"
          }
        ]
      }
    ]
  }
}
```

---

## APPENDIX B: ESCALATION CONTACTS

```yaml
escalation_matrix:
  observatory_issues:
    tier1: platform-team@company.com
    tier2: kafka-admins@company.com
    tier3: infra-lead@company.com

  registry_issues:
    tier1: platform-team@company.com
    tier2: metadata-team@company.com
    tier3: architecture-lead@company.com

  vault_issues:
    tier1: platform-team@company.com
    tier2: security-team@company.com
    tier3: ciso@company.com

  security_incidents:
    immediate: security-soc@company.com
    manager: security-manager@company.com
    executive: ciso@company.com
```

---

**Document Version:** 1.0
**Last Updated:** 2025-11-06
**Review Frequency:** Quarterly
**Next Review:** 2025-02-06
