/**
 * Knowledge Graph Builder Agent - Contract Validation Tests
 *
 * These tests verify that input/output schemas comply with agentics-contracts.
 * Tests focus on boundary conditions, invalid IDs, and constraint validation.
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';

// ============================================================================
// TYPE DEFINITIONS (matching agentics-contracts)
// ============================================================================

interface ExtractionConfig {
  extract_concepts?: boolean;
  extract_entities?: boolean;
  detect_patterns?: boolean;
  min_pattern_frequency?: number;
  max_depth?: number;
}

interface ConversationTurn {
  turn_id: string;
  role: 'user' | 'assistant' | 'system' | 'tool';
  content: string;
  timestamp: string;
}

interface ConversationData {
  session_id: string;
  turns: ConversationTurn[];
}

interface KnowledgeGraphBuilderInput {
  session_ids: string[];
  extraction_config?: ExtractionConfig;
  conversation_data?: ConversationData[];
}

interface Concept {
  concept_id: string;
  name: string;
  category?: string;
  frequency: number;
  confidence: number;
}

interface Entity {
  entity_id: string;
  name: string;
  type: string;
  mentions: number;
  confidence: number;
}

interface Relationship {
  relationship_id: string;
  source_id: string;
  target_id: string;
  type: string;
  confidence: number;
}

interface Pattern {
  pattern_id: string;
  name: string;
  description?: string;
  frequency: number;
  sessions: string[];
  confidence: number;
}

interface KnowledgeGraph {
  concepts: Concept[];
  entities: Entity[];
  relationships: Relationship[];
  patterns: Pattern[];
}

interface OutputMetadata {
  sessions_analyzed: number;
  turns_processed: number;
  concepts_extracted: number;
  entities_extracted: number;
  relationships_created: number;
  patterns_detected: number;
  extraction_timestamp: string;
}

interface KnowledgeGraphBuilderOutput {
  knowledge_graph: KnowledgeGraph;
  metadata: OutputMetadata;
}

// ============================================================================
// VALIDATION FUNCTIONS
// ============================================================================

const UUID_REGEX = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const ISO8601_REGEX = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d{3})?Z$/;

function isValidUUID(value: string): boolean {
  return UUID_REGEX.test(value);
}

function isValidTimestamp(value: string): boolean {
  return ISO8601_REGEX.test(value) && !isNaN(new Date(value).getTime());
}

function isValidConfidence(value: number): boolean {
  return typeof value === 'number' && value >= 0 && value <= 1;
}

interface ValidationResult {
  valid: boolean;
  errors: string[];
}

function validateInput(input: unknown): ValidationResult {
  const errors: string[] = [];

  if (!input || typeof input !== 'object') {
    return { valid: false, errors: ['Input must be an object'] };
  }

  const data = input as Record<string, unknown>;

  // Validate session_ids
  if (!Array.isArray(data.session_ids)) {
    errors.push('session_ids must be an array');
  } else if (data.session_ids.length === 0) {
    errors.push('session_ids cannot be empty');
  } else {
    data.session_ids.forEach((id, index) => {
      if (typeof id !== 'string' || !isValidUUID(id)) {
        errors.push(`session_ids[${index}] must be a valid UUID`);
      }
    });
  }

  // Validate extraction_config if present
  if (data.extraction_config !== undefined) {
    const config = data.extraction_config as Record<string, unknown>;

    if (config.min_pattern_frequency !== undefined) {
      if (typeof config.min_pattern_frequency !== 'number' || config.min_pattern_frequency < 1) {
        errors.push('extraction_config.min_pattern_frequency must be >= 1');
      }
    }

    if (config.max_depth !== undefined) {
      if (typeof config.max_depth !== 'number' || config.max_depth < 1 || config.max_depth > 10) {
        errors.push('extraction_config.max_depth must be between 1 and 10');
      }
    }
  }

  // Validate conversation_data if present
  if (data.conversation_data !== undefined) {
    if (!Array.isArray(data.conversation_data)) {
      errors.push('conversation_data must be an array');
    } else {
      (data.conversation_data as ConversationData[]).forEach((conv, i) => {
        if (!conv.session_id || !isValidUUID(conv.session_id)) {
          errors.push(`conversation_data[${i}].session_id must be a valid UUID`);
        }
        if (!Array.isArray(conv.turns)) {
          errors.push(`conversation_data[${i}].turns must be an array`);
        } else {
          conv.turns.forEach((turn, j) => {
            if (!turn.content || typeof turn.content !== 'string') {
              errors.push(`conversation_data[${i}].turns[${j}].content must be a non-empty string`);
            }
            if (!['user', 'assistant', 'system', 'tool'].includes(turn.role)) {
              errors.push(`conversation_data[${i}].turns[${j}].role must be a valid role`);
            }
            if (!isValidTimestamp(turn.timestamp)) {
              errors.push(`conversation_data[${i}].turns[${j}].timestamp must be ISO 8601 UTC`);
            }
          });
        }
      });
    }
  }

  return { valid: errors.length === 0, errors };
}

function validateOutput(output: unknown): ValidationResult {
  const errors: string[] = [];

  if (!output || typeof output !== 'object') {
    return { valid: false, errors: ['Output must be an object'] };
  }

  const data = output as Record<string, unknown>;

  // Validate knowledge_graph
  if (!data.knowledge_graph || typeof data.knowledge_graph !== 'object') {
    errors.push('knowledge_graph is required');
  } else {
    const kg = data.knowledge_graph as KnowledgeGraph;

    // Validate concepts
    if (!Array.isArray(kg.concepts)) {
      errors.push('knowledge_graph.concepts must be an array');
    } else {
      kg.concepts.forEach((concept, i) => {
        if (!concept.concept_id || typeof concept.concept_id !== 'string') {
          errors.push(`concepts[${i}].concept_id is required`);
        }
        if (!concept.name || typeof concept.name !== 'string') {
          errors.push(`concepts[${i}].name is required`);
        }
        if (!isValidConfidence(concept.confidence)) {
          errors.push(`concepts[${i}].confidence must be between 0 and 1`);
        }
        if (typeof concept.frequency !== 'number' || concept.frequency < 0) {
          errors.push(`concepts[${i}].frequency must be >= 0`);
        }
      });
    }

    // Validate entities
    if (!Array.isArray(kg.entities)) {
      errors.push('knowledge_graph.entities must be an array');
    } else {
      kg.entities.forEach((entity, i) => {
        if (!isValidConfidence(entity.confidence)) {
          errors.push(`entities[${i}].confidence must be between 0 and 1`);
        }
      });
    }

    // Validate relationships
    if (!Array.isArray(kg.relationships)) {
      errors.push('knowledge_graph.relationships must be an array');
    } else {
      kg.relationships.forEach((rel, i) => {
        if (!rel.source_id || !rel.target_id) {
          errors.push(`relationships[${i}] must have source_id and target_id`);
        }
        if (!isValidConfidence(rel.confidence)) {
          errors.push(`relationships[${i}].confidence must be between 0 and 1`);
        }
      });
    }

    // Validate patterns
    if (!Array.isArray(kg.patterns)) {
      errors.push('knowledge_graph.patterns must be an array');
    } else {
      kg.patterns.forEach((pattern, i) => {
        if (!isValidConfidence(pattern.confidence)) {
          errors.push(`patterns[${i}].confidence must be between 0 and 1`);
        }
        if (!Array.isArray(pattern.sessions)) {
          errors.push(`patterns[${i}].sessions must be an array`);
        }
      });
    }
  }

  // Validate metadata
  if (!data.metadata || typeof data.metadata !== 'object') {
    errors.push('metadata is required');
  } else {
    const meta = data.metadata as OutputMetadata;
    if (typeof meta.sessions_analyzed !== 'number' || meta.sessions_analyzed < 0) {
      errors.push('metadata.sessions_analyzed must be >= 0');
    }
    if (!isValidTimestamp(meta.extraction_timestamp)) {
      errors.push('metadata.extraction_timestamp must be ISO 8601 UTC');
    }
  }

  return { valid: errors.length === 0, errors };
}

// ============================================================================
// TESTS
// ============================================================================

describe('Knowledge Graph Builder - Contract Validation', () => {
  describe('Input Schema Validation', () => {
    it('should accept valid input with all fields', () => {
      const input: KnowledgeGraphBuilderInput = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        extraction_config: {
          extract_concepts: true,
          extract_entities: true,
          detect_patterns: true,
          min_pattern_frequency: 2,
          max_depth: 3,
        },
      };

      const result = validateInput(input);
      assert.strictEqual(result.valid, true, `Validation errors: ${result.errors.join(', ')}`);
    });

    it('should accept valid input with only required fields', () => {
      const input = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
      };

      const result = validateInput(input);
      assert.strictEqual(result.valid, true, `Validation errors: ${result.errors.join(', ')}`);
    });

    it('should accept input with multiple session_ids', () => {
      const input = {
        session_ids: [
          'a1b2c3d4-e5f6-4789-abcd-123456789012',
          'b2c3d4e5-f6a7-4890-bcde-234567890123',
          'c3d4e5f6-a7b8-4901-cdef-345678901234',
        ],
      };

      const result = validateInput(input);
      assert.strictEqual(result.valid, true);
    });

    it('should reject input with empty session_ids array', () => {
      const input = {
        session_ids: [],
      };

      const result = validateInput(input);
      assert.strictEqual(result.valid, false);
      assert.ok(result.errors.some((e) => e.includes('cannot be empty')));
    });

    it('should reject input with invalid UUID in session_ids', () => {
      const input = {
        session_ids: ['not-a-valid-uuid'],
      };

      const result = validateInput(input);
      assert.strictEqual(result.valid, false);
      assert.ok(result.errors.some((e) => e.includes('valid UUID')));
    });

    it('should reject input with null session_ids', () => {
      const input = {
        session_ids: null,
      };

      const result = validateInput(input);
      assert.strictEqual(result.valid, false);
    });

    it('should reject non-object input', () => {
      const result = validateInput('not an object');
      assert.strictEqual(result.valid, false);
      assert.ok(result.errors.some((e) => e.includes('must be an object')));
    });

    it('should reject null input', () => {
      const result = validateInput(null);
      assert.strictEqual(result.valid, false);
    });

    it('should reject undefined input', () => {
      const result = validateInput(undefined);
      assert.strictEqual(result.valid, false);
    });
  });

  describe('Extraction Config Validation', () => {
    it('should accept valid extraction_config', () => {
      const input = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        extraction_config: {
          extract_concepts: true,
          extract_entities: false,
          detect_patterns: true,
          min_pattern_frequency: 3,
          max_depth: 5,
        },
      };

      const result = validateInput(input);
      assert.strictEqual(result.valid, true);
    });

    it('should reject min_pattern_frequency less than 1', () => {
      const input = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        extraction_config: {
          min_pattern_frequency: 0,
        },
      };

      const result = validateInput(input);
      assert.strictEqual(result.valid, false);
      assert.ok(result.errors.some((e) => e.includes('min_pattern_frequency')));
    });

    it('should reject negative min_pattern_frequency', () => {
      const input = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        extraction_config: {
          min_pattern_frequency: -1,
        },
      };

      const result = validateInput(input);
      assert.strictEqual(result.valid, false);
    });

    it('should reject max_depth greater than 10', () => {
      const input = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        extraction_config: {
          max_depth: 11,
        },
      };

      const result = validateInput(input);
      assert.strictEqual(result.valid, false);
      assert.ok(result.errors.some((e) => e.includes('max_depth')));
    });

    it('should reject max_depth less than 1', () => {
      const input = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        extraction_config: {
          max_depth: 0,
        },
      };

      const result = validateInput(input);
      assert.strictEqual(result.valid, false);
    });

    it('should accept boundary value max_depth = 1', () => {
      const input = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        extraction_config: {
          max_depth: 1,
        },
      };

      const result = validateInput(input);
      assert.strictEqual(result.valid, true);
    });

    it('should accept boundary value max_depth = 10', () => {
      const input = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        extraction_config: {
          max_depth: 10,
        },
      };

      const result = validateInput(input);
      assert.strictEqual(result.valid, true);
    });
  });

  describe('Conversation Data Validation', () => {
    it('should accept valid conversation_data', () => {
      const input = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        conversation_data: [
          {
            session_id: 'a1b2c3d4-e5f6-4789-abcd-123456789012',
            turns: [
              {
                turn_id: 'turn-001-uuid-1234-5678-abcdefabcdef',
                role: 'user',
                content: 'Hello',
                timestamp: '2024-01-15T10:00:00.000Z',
              },
            ],
          },
        ],
      };

      const result = validateInput(input);
      assert.strictEqual(result.valid, true);
    });

    it('should reject invalid session_id in conversation_data', () => {
      const input = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        conversation_data: [
          {
            session_id: 'invalid-session-id',
            turns: [],
          },
        ],
      };

      const result = validateInput(input);
      assert.strictEqual(result.valid, false);
    });

    it('should reject invalid role in turns', () => {
      const input = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        conversation_data: [
          {
            session_id: 'a1b2c3d4-e5f6-4789-abcd-123456789012',
            turns: [
              {
                turn_id: 'turn-001',
                role: 'invalid_role',
                content: 'Hello',
                timestamp: '2024-01-15T10:00:00.000Z',
              },
            ],
          },
        ],
      };

      const result = validateInput(input);
      assert.strictEqual(result.valid, false);
      assert.ok(result.errors.some((e) => e.includes('valid role')));
    });

    it('should reject invalid timestamp format', () => {
      const input = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        conversation_data: [
          {
            session_id: 'a1b2c3d4-e5f6-4789-abcd-123456789012',
            turns: [
              {
                turn_id: 'turn-001',
                role: 'user',
                content: 'Hello',
                timestamp: '2024-01-15 10:00:00', // Not ISO 8601 UTC
              },
            ],
          },
        ],
      };

      const result = validateInput(input);
      assert.strictEqual(result.valid, false);
      assert.ok(result.errors.some((e) => e.includes('ISO 8601')));
    });

    it('should reject empty content in turns', () => {
      const input = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        conversation_data: [
          {
            session_id: 'a1b2c3d4-e5f6-4789-abcd-123456789012',
            turns: [
              {
                turn_id: 'turn-001',
                role: 'user',
                content: '',
                timestamp: '2024-01-15T10:00:00.000Z',
              },
            ],
          },
        ],
      };

      const result = validateInput(input);
      assert.strictEqual(result.valid, false);
    });

    it('should accept all valid roles', () => {
      const roles = ['user', 'assistant', 'system', 'tool'];

      for (const role of roles) {
        const input = {
          session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
          conversation_data: [
            {
              session_id: 'a1b2c3d4-e5f6-4789-abcd-123456789012',
              turns: [
                {
                  turn_id: 'turn-001',
                  role,
                  content: 'Test content',
                  timestamp: '2024-01-15T10:00:00.000Z',
                },
              ],
            },
          ],
        };

        const result = validateInput(input);
        assert.strictEqual(result.valid, true, `Role '${role}' should be valid`);
      }
    });
  });

  describe('Output Schema Validation', () => {
    it('should accept valid output with all fields', () => {
      const output: KnowledgeGraphBuilderOutput = {
        knowledge_graph: {
          concepts: [
            {
              concept_id: 'concept-001',
              name: 'Test Concept',
              category: 'test',
              frequency: 5,
              confidence: 0.95,
            },
          ],
          entities: [
            {
              entity_id: 'entity-001',
              name: 'Test Entity',
              type: 'library',
              mentions: 3,
              confidence: 0.88,
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
              sessions: ['session-001'],
              confidence: 0.82,
            },
          ],
        },
        metadata: {
          sessions_analyzed: 1,
          turns_processed: 10,
          concepts_extracted: 1,
          entities_extracted: 1,
          relationships_created: 1,
          patterns_detected: 1,
          extraction_timestamp: '2024-01-15T12:00:00.000Z',
        },
      };

      const result = validateOutput(output);
      assert.strictEqual(result.valid, true, `Validation errors: ${result.errors.join(', ')}`);
    });

    it('should accept output with empty arrays', () => {
      const output = {
        knowledge_graph: {
          concepts: [],
          entities: [],
          relationships: [],
          patterns: [],
        },
        metadata: {
          sessions_analyzed: 0,
          turns_processed: 0,
          concepts_extracted: 0,
          entities_extracted: 0,
          relationships_created: 0,
          patterns_detected: 0,
          extraction_timestamp: '2024-01-15T12:00:00.000Z',
        },
      };

      const result = validateOutput(output);
      assert.strictEqual(result.valid, true);
    });

    it('should reject output with confidence > 1', () => {
      const output = {
        knowledge_graph: {
          concepts: [
            {
              concept_id: 'concept-001',
              name: 'Test',
              frequency: 1,
              confidence: 1.5, // Invalid
            },
          ],
          entities: [],
          relationships: [],
          patterns: [],
        },
        metadata: {
          sessions_analyzed: 1,
          turns_processed: 1,
          concepts_extracted: 1,
          entities_extracted: 0,
          relationships_created: 0,
          patterns_detected: 0,
          extraction_timestamp: '2024-01-15T12:00:00.000Z',
        },
      };

      const result = validateOutput(output);
      assert.strictEqual(result.valid, false);
      assert.ok(result.errors.some((e) => e.includes('confidence')));
    });

    it('should reject output with confidence < 0', () => {
      const output = {
        knowledge_graph: {
          concepts: [
            {
              concept_id: 'concept-001',
              name: 'Test',
              frequency: 1,
              confidence: -0.1, // Invalid
            },
          ],
          entities: [],
          relationships: [],
          patterns: [],
        },
        metadata: {
          sessions_analyzed: 1,
          turns_processed: 1,
          concepts_extracted: 1,
          entities_extracted: 0,
          relationships_created: 0,
          patterns_detected: 0,
          extraction_timestamp: '2024-01-15T12:00:00.000Z',
        },
      };

      const result = validateOutput(output);
      assert.strictEqual(result.valid, false);
    });

    it('should accept boundary confidence values 0.0 and 1.0', () => {
      const output = {
        knowledge_graph: {
          concepts: [
            { concept_id: 'c1', name: 'Low Confidence', frequency: 1, confidence: 0.0 },
            { concept_id: 'c2', name: 'High Confidence', frequency: 1, confidence: 1.0 },
          ],
          entities: [],
          relationships: [],
          patterns: [],
        },
        metadata: {
          sessions_analyzed: 1,
          turns_processed: 1,
          concepts_extracted: 2,
          entities_extracted: 0,
          relationships_created: 0,
          patterns_detected: 0,
          extraction_timestamp: '2024-01-15T12:00:00.000Z',
        },
      };

      const result = validateOutput(output);
      assert.strictEqual(result.valid, true);
    });

    it('should reject output with missing knowledge_graph', () => {
      const output = {
        metadata: {
          sessions_analyzed: 1,
          turns_processed: 1,
          concepts_extracted: 0,
          entities_extracted: 0,
          relationships_created: 0,
          patterns_detected: 0,
          extraction_timestamp: '2024-01-15T12:00:00.000Z',
        },
      };

      const result = validateOutput(output);
      assert.strictEqual(result.valid, false);
      assert.ok(result.errors.some((e) => e.includes('knowledge_graph')));
    });

    it('should reject output with missing metadata', () => {
      const output = {
        knowledge_graph: {
          concepts: [],
          entities: [],
          relationships: [],
          patterns: [],
        },
      };

      const result = validateOutput(output);
      assert.strictEqual(result.valid, false);
      assert.ok(result.errors.some((e) => e.includes('metadata')));
    });

    it('should reject output with invalid extraction_timestamp', () => {
      const output = {
        knowledge_graph: {
          concepts: [],
          entities: [],
          relationships: [],
          patterns: [],
        },
        metadata: {
          sessions_analyzed: 0,
          turns_processed: 0,
          concepts_extracted: 0,
          entities_extracted: 0,
          relationships_created: 0,
          patterns_detected: 0,
          extraction_timestamp: 'not-a-timestamp',
        },
      };

      const result = validateOutput(output);
      assert.strictEqual(result.valid, false);
    });

    it('should reject relationships with missing source_id or target_id', () => {
      const output = {
        knowledge_graph: {
          concepts: [],
          entities: [],
          relationships: [
            {
              relationship_id: 'rel-001',
              source_id: '', // Empty
              target_id: 'entity-001',
              type: 'uses',
              confidence: 0.8,
            },
          ],
          patterns: [],
        },
        metadata: {
          sessions_analyzed: 1,
          turns_processed: 1,
          concepts_extracted: 0,
          entities_extracted: 0,
          relationships_created: 1,
          patterns_detected: 0,
          extraction_timestamp: '2024-01-15T12:00:00.000Z',
        },
      };

      const result = validateOutput(output);
      assert.strictEqual(result.valid, false);
      assert.ok(result.errors.some((e) => e.includes('source_id') || e.includes('target_id')));
    });

    it('should reject negative frequency values', () => {
      const output = {
        knowledge_graph: {
          concepts: [
            {
              concept_id: 'concept-001',
              name: 'Test',
              frequency: -1, // Invalid
              confidence: 0.9,
            },
          ],
          entities: [],
          relationships: [],
          patterns: [],
        },
        metadata: {
          sessions_analyzed: 1,
          turns_processed: 1,
          concepts_extracted: 1,
          entities_extracted: 0,
          relationships_created: 0,
          patterns_detected: 0,
          extraction_timestamp: '2024-01-15T12:00:00.000Z',
        },
      };

      const result = validateOutput(output);
      assert.strictEqual(result.valid, false);
    });
  });
});
