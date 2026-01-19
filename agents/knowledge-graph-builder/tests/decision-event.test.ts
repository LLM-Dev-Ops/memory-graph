/**
 * Knowledge Graph Builder Agent - DecisionEvent Tests
 *
 * These tests verify DecisionEvent structure validity including:
 * - inputs_hash generation (SHA-256)
 * - timestamp format (ISO 8601 UTC)
 * - agent_version semver format
 * - Required field presence and types
 * - DecisionEvent emission rules
 */

import { describe, it, beforeEach, mock } from 'node:test';
import assert from 'node:assert';
import { createHash } from 'node:crypto';

// ============================================================================
// DECISION EVENT TYPES (matching agentics-contracts)
// ============================================================================

interface KnowledgeGraphBuilderOutput {
  knowledge_graph: {
    concepts: Array<{
      concept_id: string;
      name: string;
      category?: string;
      frequency: number;
      confidence: number;
    }>;
    entities: Array<{
      entity_id: string;
      name: string;
      type: string;
      mentions: number;
      confidence: number;
    }>;
    relationships: Array<{
      relationship_id: string;
      source_id: string;
      target_id: string;
      type: string;
      confidence: number;
    }>;
    patterns: Array<{
      pattern_id: string;
      name: string;
      description?: string;
      frequency: number;
      sessions: string[];
      confidence: number;
    }>;
  };
  metadata: {
    sessions_analyzed: number;
    turns_processed: number;
    concepts_extracted: number;
    entities_extracted: number;
    relationships_created: number;
    patterns_detected: number;
    extraction_timestamp: string;
  };
}

interface DecisionEvent {
  agent_id: string;
  agent_version: string;
  decision_type: 'pattern_analysis';
  inputs_hash: string;
  outputs: KnowledgeGraphBuilderOutput;
  confidence: number;
  constraints_applied?: string[];
  execution_ref: string;
  timestamp: string;
  telemetry?: {
    duration_ms?: number;
    memory_bytes?: number;
    ruvector_latency_ms?: number;
  };
}

// ============================================================================
// CONSTANTS
// ============================================================================

const AGENT_ID = 'knowledge-graph-builder-agent';
const AGENT_VERSION = '1.0.0';
const DECISION_TYPE = 'pattern_analysis' as const;

// ============================================================================
// DECISION EVENT FUNCTIONS
// ============================================================================

/**
 * Generate SHA-256 hash of input
 */
function hashInput(input: unknown): string {
  const data = JSON.stringify(input);
  return createHash('sha256').update(data).digest('hex');
}

/**
 * Generate UUID v4
 */
function generateUUID(): string {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === 'x' ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

/**
 * Get current UTC timestamp in ISO 8601 format
 */
function utcNow(): string {
  return new Date().toISOString();
}

/**
 * Validate semver format
 */
function isValidSemver(version: string): boolean {
  const semverRegex = /^\d+\.\d+\.\d+(-[a-zA-Z0-9.]+)?(\+[a-zA-Z0-9.]+)?$/;
  return semverRegex.test(version);
}

/**
 * Validate UUID format
 */
function isValidUUID(uuid: string): boolean {
  const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
  return uuidRegex.test(uuid);
}

/**
 * Validate ISO 8601 UTC timestamp format
 */
function isValidISO8601UTC(timestamp: string): boolean {
  const iso8601Regex = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d{3})?Z$/;
  if (!iso8601Regex.test(timestamp)) return false;

  const date = new Date(timestamp);
  return !isNaN(date.getTime());
}

/**
 * Validate SHA-256 hash format
 */
function isValidSHA256(hash: string): boolean {
  const sha256Regex = /^[a-f0-9]{64}$/;
  return sha256Regex.test(hash);
}

/**
 * Create a DecisionEvent for the Knowledge Graph Builder Agent
 */
function createDecisionEvent(
  input: unknown,
  output: KnowledgeGraphBuilderOutput,
  confidence: number,
  constraintsApplied?: string[]
): DecisionEvent {
  return {
    agent_id: AGENT_ID,
    agent_version: AGENT_VERSION,
    decision_type: DECISION_TYPE,
    inputs_hash: hashInput(input),
    outputs: output,
    confidence: Math.max(0, Math.min(1, confidence)),
    constraints_applied: constraintsApplied,
    execution_ref: generateUUID(),
    timestamp: utcNow(),
  };
}

/**
 * Validate a DecisionEvent structure
 */
interface ValidationResult {
  valid: boolean;
  errors: string[];
}

function validateDecisionEvent(event: unknown): ValidationResult {
  const errors: string[] = [];

  if (!event || typeof event !== 'object') {
    return { valid: false, errors: ['DecisionEvent must be an object'] };
  }

  const e = event as Record<string, unknown>;

  // Required fields
  if (e.agent_id !== AGENT_ID) {
    errors.push(`agent_id must be '${AGENT_ID}'`);
  }

  if (typeof e.agent_version !== 'string' || !isValidSemver(e.agent_version)) {
    errors.push('agent_version must be a valid semver string');
  }

  if (e.decision_type !== DECISION_TYPE) {
    errors.push(`decision_type must be '${DECISION_TYPE}'`);
  }

  if (typeof e.inputs_hash !== 'string' || !isValidSHA256(e.inputs_hash)) {
    errors.push('inputs_hash must be a valid SHA-256 hash (64 hex characters)');
  }

  if (!e.outputs || typeof e.outputs !== 'object') {
    errors.push('outputs is required and must be an object');
  }

  if (typeof e.confidence !== 'number' || e.confidence < 0 || e.confidence > 1) {
    errors.push('confidence must be a number between 0 and 1');
  }

  if (typeof e.execution_ref !== 'string' || !isValidUUID(e.execution_ref)) {
    errors.push('execution_ref must be a valid UUID');
  }

  if (typeof e.timestamp !== 'string' || !isValidISO8601UTC(e.timestamp)) {
    errors.push('timestamp must be a valid ISO 8601 UTC timestamp');
  }

  // Optional fields
  if (e.constraints_applied !== undefined) {
    if (!Array.isArray(e.constraints_applied)) {
      errors.push('constraints_applied must be an array if present');
    } else {
      for (let i = 0; i < e.constraints_applied.length; i++) {
        if (typeof e.constraints_applied[i] !== 'string') {
          errors.push(`constraints_applied[${i}] must be a string`);
        }
      }
    }
  }

  if (e.telemetry !== undefined) {
    if (typeof e.telemetry !== 'object') {
      errors.push('telemetry must be an object if present');
    }
  }

  return { valid: errors.length === 0, errors };
}

// ============================================================================
// MOCK OUTPUT GENERATOR
// ============================================================================

function createMockOutput(): KnowledgeGraphBuilderOutput {
  return {
    knowledge_graph: {
      concepts: [
        {
          concept_id: 'concept-001',
          name: 'Test Concept',
          category: 'test',
          frequency: 5,
          confidence: 0.9,
        },
      ],
      entities: [
        {
          entity_id: 'entity-001',
          name: 'Test Entity',
          type: 'library',
          mentions: 3,
          confidence: 0.85,
        },
      ],
      relationships: [
        {
          relationship_id: 'rel-001',
          source_id: 'concept-001',
          target_id: 'entity-001',
          type: 'uses',
          confidence: 0.75,
        },
      ],
      patterns: [
        {
          pattern_id: 'pattern-001',
          name: 'Test Pattern',
          description: 'A test pattern',
          frequency: 2,
          sessions: ['session-001', 'session-002'],
          confidence: 0.8,
        },
      ],
    },
    metadata: {
      sessions_analyzed: 2,
      turns_processed: 10,
      concepts_extracted: 1,
      entities_extracted: 1,
      relationships_created: 1,
      patterns_detected: 1,
      extraction_timestamp: utcNow(),
    },
  };
}

// ============================================================================
// TESTS
// ============================================================================

describe('Knowledge Graph Builder - DecisionEvent', () => {
  describe('inputs_hash Generation (SHA-256)', () => {
    it('should generate valid SHA-256 hash', () => {
      const input = { session_ids: ['test-session'] };
      const hash = hashInput(input);

      assert.ok(isValidSHA256(hash), 'Hash should be valid SHA-256');
      assert.strictEqual(hash.length, 64, 'SHA-256 hash should be 64 hex characters');
    });

    it('should produce consistent hash for same input', () => {
      const input = { session_ids: ['session-1', 'session-2'], config: { maxDepth: 3 } };

      const hash1 = hashInput(input);
      const hash2 = hashInput(input);

      assert.strictEqual(hash1, hash2, 'Same input should produce identical hash');
    });

    it('should produce different hash for different input', () => {
      const input1 = { session_ids: ['session-1'] };
      const input2 = { session_ids: ['session-2'] };

      const hash1 = hashInput(input1);
      const hash2 = hashInput(input2);

      assert.notStrictEqual(hash1, hash2, 'Different input should produce different hash');
    });

    it('should handle complex nested input', () => {
      const complexInput = {
        session_ids: ['s1', 's2', 's3'],
        extraction_config: {
          extract_concepts: true,
          extract_entities: true,
          detect_patterns: true,
          min_pattern_frequency: 2,
          max_depth: 5,
        },
        conversation_data: [
          {
            session_id: 's1',
            turns: [
              { turn_id: 't1', role: 'user', content: 'Hello', timestamp: '2024-01-15T10:00:00.000Z' },
            ],
          },
        ],
      };

      const hash = hashInput(complexInput);
      assert.ok(isValidSHA256(hash));
    });

    it('should handle empty input', () => {
      const hash = hashInput({});
      assert.ok(isValidSHA256(hash));
    });

    it('should handle null and undefined values in input', () => {
      const inputWithNull = { value: null };
      const inputWithUndefined = { value: undefined };

      const hashNull = hashInput(inputWithNull);
      const hashUndefined = hashInput(inputWithUndefined);

      assert.ok(isValidSHA256(hashNull));
      assert.ok(isValidSHA256(hashUndefined));
      // Note: JSON.stringify treats null and undefined differently
    });

    it('should be lowercase hexadecimal', () => {
      const hash = hashInput({ test: 'data' });
      assert.ok(/^[a-f0-9]+$/.test(hash), 'Hash should contain only lowercase hex characters');
    });
  });

  describe('Timestamp Format (ISO 8601 UTC)', () => {
    it('should generate valid ISO 8601 UTC timestamp', () => {
      const timestamp = utcNow();
      assert.ok(isValidISO8601UTC(timestamp), `Timestamp '${timestamp}' should be valid ISO 8601 UTC`);
    });

    it('should end with Z (UTC indicator)', () => {
      const timestamp = utcNow();
      assert.ok(timestamp.endsWith('Z'), 'Timestamp should end with Z');
    });

    it('should include milliseconds', () => {
      const timestamp = utcNow();
      assert.ok(timestamp.includes('.'), 'Timestamp should include milliseconds');
    });

    it('should reject non-UTC timestamps', () => {
      const nonUTC = '2024-01-15T10:00:00+05:00';
      assert.ok(!isValidISO8601UTC(nonUTC), 'Non-UTC timestamp should be invalid');
    });

    it('should reject invalid date formats', () => {
      const invalidFormats = [
        '2024-01-15 10:00:00Z', // Space instead of T
        '2024/01/15T10:00:00Z', // Wrong date separator
        '2024-1-15T10:00:00Z', // Single digit month
        '15-01-2024T10:00:00Z', // DD-MM-YYYY format
        '2024-01-15T10:00:00', // Missing Z
        'not-a-timestamp',
      ];

      for (const invalid of invalidFormats) {
        assert.ok(!isValidISO8601UTC(invalid), `'${invalid}' should be invalid`);
      }
    });

    it('should accept valid timestamps without milliseconds', () => {
      const timestamp = '2024-01-15T10:00:00Z';
      assert.ok(isValidISO8601UTC(timestamp));
    });

    it('should parse to valid Date object', () => {
      const timestamp = utcNow();
      const date = new Date(timestamp);
      assert.ok(!isNaN(date.getTime()), 'Timestamp should parse to valid Date');
    });
  });

  describe('agent_version Semver Format', () => {
    it('should be valid semver', () => {
      assert.ok(isValidSemver(AGENT_VERSION), `'${AGENT_VERSION}' should be valid semver`);
    });

    it('should accept standard semver versions', () => {
      const validVersions = ['1.0.0', '2.3.4', '0.0.1', '10.20.30', '1.2.3-alpha', '1.2.3+build'];

      for (const version of validVersions) {
        assert.ok(isValidSemver(version), `'${version}' should be valid semver`);
      }
    });

    it('should reject invalid semver versions', () => {
      const invalidVersions = [
        '1.0', // Missing patch
        '1', // Missing minor and patch
        'v1.0.0', // Leading v
        '1.0.0.0', // Extra component
        '1.a.0', // Non-numeric
        '', // Empty
        'latest',
      ];

      for (const version of invalidVersions) {
        assert.ok(!isValidSemver(version), `'${version}' should be invalid semver`);
      }
    });
  });

  describe('DecisionEvent Structure', () => {
    it('should create valid DecisionEvent with all required fields', () => {
      const input = { session_ids: ['session-1'] };
      const output = createMockOutput();
      const event = createDecisionEvent(input, output, 0.85);

      const validation = validateDecisionEvent(event);
      assert.ok(validation.valid, `Validation errors: ${validation.errors.join(', ')}`);
    });

    it('should include correct agent_id', () => {
      const event = createDecisionEvent({}, createMockOutput(), 0.9);
      assert.strictEqual(event.agent_id, AGENT_ID);
    });

    it('should include correct decision_type', () => {
      const event = createDecisionEvent({}, createMockOutput(), 0.9);
      assert.strictEqual(event.decision_type, DECISION_TYPE);
    });

    it('should include valid execution_ref UUID', () => {
      const event = createDecisionEvent({}, createMockOutput(), 0.9);
      assert.ok(isValidUUID(event.execution_ref));
    });

    it('should generate unique execution_ref for each invocation', () => {
      const event1 = createDecisionEvent({}, createMockOutput(), 0.9);
      const event2 = createDecisionEvent({}, createMockOutput(), 0.9);

      assert.notStrictEqual(
        event1.execution_ref,
        event2.execution_ref,
        'Each invocation should have unique execution_ref'
      );
    });

    it('should clamp confidence to valid range', () => {
      const eventLow = createDecisionEvent({}, createMockOutput(), -0.5);
      const eventHigh = createDecisionEvent({}, createMockOutput(), 1.5);

      assert.strictEqual(eventLow.confidence, 0, 'Negative confidence should clamp to 0');
      assert.strictEqual(eventHigh.confidence, 1, 'Confidence > 1 should clamp to 1');
    });

    it('should include constraints_applied when provided', () => {
      const constraints = ['max_depth_limit', 'session_filter'];
      const event = createDecisionEvent({}, createMockOutput(), 0.9, constraints);

      assert.deepStrictEqual(event.constraints_applied, constraints);
    });

    it('should not include constraints_applied when not provided', () => {
      const event = createDecisionEvent({}, createMockOutput(), 0.9);
      assert.strictEqual(event.constraints_applied, undefined);
    });
  });

  describe('DecisionEvent Validation', () => {
    it('should reject event with wrong agent_id', () => {
      const event = {
        ...createDecisionEvent({}, createMockOutput(), 0.9),
        agent_id: 'wrong-agent-id',
      };

      const validation = validateDecisionEvent(event);
      assert.ok(!validation.valid);
      assert.ok(validation.errors.some((e) => e.includes('agent_id')));
    });

    it('should reject event with invalid agent_version', () => {
      const event = {
        ...createDecisionEvent({}, createMockOutput(), 0.9),
        agent_version: 'invalid',
      };

      const validation = validateDecisionEvent(event);
      assert.ok(!validation.valid);
      assert.ok(validation.errors.some((e) => e.includes('agent_version')));
    });

    it('should reject event with invalid inputs_hash', () => {
      const event = {
        ...createDecisionEvent({}, createMockOutput(), 0.9),
        inputs_hash: 'not-a-valid-hash',
      };

      const validation = validateDecisionEvent(event);
      assert.ok(!validation.valid);
      assert.ok(validation.errors.some((e) => e.includes('inputs_hash')));
    });

    it('should reject event with missing outputs', () => {
      const event = createDecisionEvent({}, createMockOutput(), 0.9);
      delete (event as Partial<DecisionEvent>).outputs;

      const validation = validateDecisionEvent(event);
      assert.ok(!validation.valid);
      assert.ok(validation.errors.some((e) => e.includes('outputs')));
    });

    it('should reject event with confidence out of range', () => {
      const event = {
        ...createDecisionEvent({}, createMockOutput(), 0.9),
        confidence: 1.5,
      };

      const validation = validateDecisionEvent(event);
      assert.ok(!validation.valid);
      assert.ok(validation.errors.some((e) => e.includes('confidence')));
    });

    it('should reject event with invalid execution_ref', () => {
      const event = {
        ...createDecisionEvent({}, createMockOutput(), 0.9),
        execution_ref: 'not-a-uuid',
      };

      const validation = validateDecisionEvent(event);
      assert.ok(!validation.valid);
      assert.ok(validation.errors.some((e) => e.includes('execution_ref')));
    });

    it('should reject event with invalid timestamp', () => {
      const event = {
        ...createDecisionEvent({}, createMockOutput(), 0.9),
        timestamp: 'invalid-timestamp',
      };

      const validation = validateDecisionEvent(event);
      assert.ok(!validation.valid);
      assert.ok(validation.errors.some((e) => e.includes('timestamp')));
    });

    it('should reject non-string items in constraints_applied', () => {
      const event = {
        ...createDecisionEvent({}, createMockOutput(), 0.9),
        constraints_applied: ['valid', 123, 'also-valid'],
      };

      const validation = validateDecisionEvent(event);
      assert.ok(!validation.valid);
    });
  });

  describe('DecisionEvent Emission Rules', () => {
    it('should emit exactly ONE DecisionEvent structure per createDecisionEvent call', () => {
      // This test verifies the contract that each agent invocation produces
      // exactly one DecisionEvent
      const input = { session_ids: ['session-1'] };
      const output = createMockOutput();

      const event = createDecisionEvent(input, output, 0.9);

      // Verify it's a single object, not an array
      assert.ok(!Array.isArray(event), 'Should return single event, not array');
      assert.ok(typeof event === 'object', 'Should return an object');

      // Verify all required fields exist
      assert.ok(event.agent_id, 'Should have agent_id');
      assert.ok(event.agent_version, 'Should have agent_version');
      assert.ok(event.decision_type, 'Should have decision_type');
      assert.ok(event.inputs_hash, 'Should have inputs_hash');
      assert.ok(event.outputs, 'Should have outputs');
      assert.ok(typeof event.confidence === 'number', 'Should have confidence');
      assert.ok(event.execution_ref, 'Should have execution_ref');
      assert.ok(event.timestamp, 'Should have timestamp');
    });

    it('should produce deterministic outputs for same input (excluding execution_ref and timestamp)', () => {
      const input = { session_ids: ['session-1'] };
      const output = createMockOutput();

      const event1 = createDecisionEvent(input, output, 0.9);
      const event2 = createDecisionEvent(input, output, 0.9);

      // These should be identical
      assert.strictEqual(event1.agent_id, event2.agent_id);
      assert.strictEqual(event1.agent_version, event2.agent_version);
      assert.strictEqual(event1.decision_type, event2.decision_type);
      assert.strictEqual(event1.inputs_hash, event2.inputs_hash);
      assert.strictEqual(event1.confidence, event2.confidence);

      // These should differ (non-deterministic)
      // execution_ref and timestamp are generated fresh each time
    });

    it('should NOT modify runtime execution (outputs reflect analysis only)', () => {
      const input = { session_ids: ['session-1'] };
      const output = createMockOutput();

      const event = createDecisionEvent(input, output, 0.9);

      // The DecisionEvent should contain analysis results only
      // It should NOT contain:
      // - Action commands
      // - Side effect triggers
      // - Policy enforcement flags
      // - Orchestration instructions

      const eventStr = JSON.stringify(event);

      // Verify no action-related keywords that would indicate policy enforcement
      const prohibitedKeywords = [
        'enforce',
        'execute',
        'trigger',
        'action',
        'command',
        'remediate',
        'alert',
        'block',
        'deny',
      ];

      for (const keyword of prohibitedKeywords) {
        // Allow these words in natural descriptions, but not as field names
        const hasProhibitedField = Object.keys(event).some(
          (key) => key.toLowerCase().includes(keyword)
        );
        assert.ok(
          !hasProhibitedField,
          `DecisionEvent should not have field containing '${keyword}'`
        );
      }
    });
  });
});
