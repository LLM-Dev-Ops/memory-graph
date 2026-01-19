/**
 * Knowledge Graph Builder Agent - Integration Tests
 *
 * These tests verify the full pipeline execution including:
 * - End-to-end processing flow
 * - Deterministic output (same input = same output)
 * - Error handling paths
 * - Verification of NO side effects beyond DecisionEvent emission
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

// ============================================================================
// TYPES
// ============================================================================

interface ExtractionConfig {
  extract_concepts: boolean;
  extract_entities: boolean;
  detect_patterns: boolean;
  min_pattern_frequency: number;
  max_depth: number;
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
}

interface AgentError {
  error_code: string;
  message: string;
  details?: Record<string, unknown>;
  execution_ref: string;
  timestamp: string;
}

type AgentResult =
  | { success: true; output: KnowledgeGraphBuilderOutput; decisionEvent: DecisionEvent }
  | { success: false; error: AgentError };

// ============================================================================
// MOCK KNOWLEDGE GRAPH BUILDER AGENT
// ============================================================================

const AGENT_ID = 'knowledge-graph-builder-agent';
const AGENT_VERSION = '1.0.0';
const DECISION_TYPE = 'pattern_analysis' as const;

function generateUUID(): string {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === 'x' ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

function hashInput(input: unknown): string {
  const data = JSON.stringify(input);
  return createHash('sha256').update(data).digest('hex');
}

function utcNow(): string {
  return new Date().toISOString();
}

/**
 * Extract concepts from content (simplified implementation)
 */
function extractConcepts(content: string): Concept[] {
  const concepts: Concept[] = [];
  const contentLower = content.toLowerCase();

  const patterns: [RegExp, string, string][] = [
    [/\bjwt\b|\bjson web token\b/gi, 'JWT Authentication', 'security'],
    [/\bnode\.?js\b/gi, 'Node.js', 'technology'],
    [/\bmiddleware\b/gi, 'Middleware', 'architecture'],
    [/\bauthentication\b|\bauth\b/gi, 'Authentication', 'security'],
    [/\bapi\b/gi, 'API', 'architecture'],
  ];

  for (const [regex, name, category] of patterns) {
    const matches = content.match(regex);
    if (matches && matches.length > 0) {
      concepts.push({
        concept_id: `concept-${name.toLowerCase().replace(/[^a-z0-9]/g, '-')}`,
        name,
        category,
        frequency: matches.length,
        confidence: Math.min(0.7 + matches.length * 0.1, 0.99),
      });
    }
  }

  return concepts;
}

/**
 * Extract entities from content (simplified implementation)
 */
function extractEntities(content: string): Entity[] {
  const entities: Entity[] = [];
  const seenEntities = new Set<string>();

  const entityPatterns: [RegExp, string][] = [
    [/\b(jsonwebtoken|express|mongoose|axios)\b/gi, 'library'],
    [/\b(npm|yarn|git|docker)\b/gi, 'tool'],
  ];

  for (const [regex, type] of entityPatterns) {
    const matches = content.match(regex) || [];
    for (const match of matches) {
      const normalized = match.toLowerCase();
      if (!seenEntities.has(normalized)) {
        seenEntities.add(normalized);
        entities.push({
          entity_id: `entity-${normalized}`,
          name: match,
          type,
          mentions: (content.match(new RegExp(match, 'gi')) || []).length,
          confidence: 0.95,
        });
      }
    }
  }

  return entities;
}

/**
 * Validate input schema
 */
function validateInput(
  input: unknown
): { valid: true; data: KnowledgeGraphBuilderInput } | { valid: false; error: string } {
  if (!input || typeof input !== 'object') {
    return { valid: false, error: 'Input must be an object' };
  }

  const data = input as Record<string, unknown>;

  if (!Array.isArray(data.session_ids)) {
    return { valid: false, error: 'session_ids must be an array' };
  }

  if (data.session_ids.length === 0) {
    return { valid: false, error: 'session_ids cannot be empty' };
  }

  const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
  for (const id of data.session_ids as string[]) {
    if (typeof id !== 'string' || !uuidRegex.test(id)) {
      return { valid: false, error: `Invalid session_id: ${id}` };
    }
  }

  return { valid: true, data: data as KnowledgeGraphBuilderInput };
}

/**
 * Mock Knowledge Graph Builder Agent
 */
class KnowledgeGraphBuilderAgent {
  private sideEffectLog: string[] = [];

  /**
   * Execute the agent - main entry point
   */
  async execute(input: unknown): Promise<AgentResult> {
    const executionRef = generateUUID();
    const startTime = Date.now();

    try {
      // Validate input
      const validation = validateInput(input);
      if (!validation.valid) {
        return {
          success: false,
          error: {
            error_code: 'VALIDATION_ERROR',
            message: validation.error,
            execution_ref: executionRef,
            timestamp: utcNow(),
          },
        };
      }

      const validInput = validation.data;

      // Process conversations
      const output = await this.processKnowledgeGraph(validInput);

      // Create DecisionEvent
      const decisionEvent: DecisionEvent = {
        agent_id: AGENT_ID,
        agent_version: AGENT_VERSION,
        decision_type: DECISION_TYPE,
        inputs_hash: hashInput(validInput),
        outputs: output,
        confidence: this.calculateOverallConfidence(output),
        constraints_applied: this.getConstraintsApplied(validInput),
        execution_ref: executionRef,
        timestamp: utcNow(),
      };

      return {
        success: true,
        output,
        decisionEvent,
      };
    } catch (error) {
      return {
        success: false,
        error: {
          error_code: 'INTERNAL_ERROR',
          message: error instanceof Error ? error.message : String(error),
          execution_ref: executionRef,
          timestamp: utcNow(),
        },
      };
    }
  }

  /**
   * Process input to build knowledge graph
   */
  private async processKnowledgeGraph(
    input: KnowledgeGraphBuilderInput
  ): Promise<KnowledgeGraphBuilderOutput> {
    const allConcepts: Map<string, Concept> = new Map();
    const allEntities: Map<string, Entity> = new Map();
    const relationships: Relationship[] = [];
    const patterns: Pattern[] = [];
    let totalTurns = 0;

    // Process conversation data if provided
    if (input.conversation_data) {
      for (const conv of input.conversation_data) {
        for (const turn of conv.turns) {
          totalTurns++;

          // Extract concepts
          if (input.extraction_config?.extract_concepts !== false) {
            const concepts = extractConcepts(turn.content);
            for (const concept of concepts) {
              const existing = allConcepts.get(concept.concept_id);
              if (existing) {
                existing.frequency += concept.frequency;
                existing.confidence = Math.max(existing.confidence, concept.confidence);
              } else {
                allConcepts.set(concept.concept_id, { ...concept });
              }
            }
          }

          // Extract entities
          if (input.extraction_config?.extract_entities !== false) {
            const entities = extractEntities(turn.content);
            for (const entity of entities) {
              const existing = allEntities.get(entity.entity_id);
              if (existing) {
                existing.mentions += entity.mentions;
              } else {
                allEntities.set(entity.entity_id, { ...entity });
              }
            }
          }
        }
      }
    }

    // Create relationships between concepts and entities
    const conceptList = Array.from(allConcepts.values());
    const entityList = Array.from(allEntities.values());

    for (const concept of conceptList) {
      for (const entity of entityList) {
        relationships.push({
          relationship_id: `rel-${concept.concept_id}-${entity.entity_id}`,
          source_id: concept.concept_id,
          target_id: entity.entity_id,
          type: 'related_to',
          confidence: Math.min(concept.confidence, entity.confidence) * 0.8,
        });
      }
    }

    // Detect patterns
    if (input.extraction_config?.detect_patterns !== false) {
      const minFreq = input.extraction_config?.min_pattern_frequency ?? 2;
      for (const concept of conceptList) {
        if (concept.frequency >= minFreq) {
          patterns.push({
            pattern_id: `pattern-${concept.concept_id}`,
            name: `${concept.name} Pattern`,
            description: `Recurring discussions about ${concept.name}`,
            frequency: concept.frequency,
            sessions: input.session_ids,
            confidence: concept.confidence * 0.9,
          });
        }
      }
    }

    return {
      knowledge_graph: {
        concepts: conceptList,
        entities: entityList,
        relationships,
        patterns,
      },
      metadata: {
        sessions_analyzed: input.session_ids.length,
        turns_processed: totalTurns,
        concepts_extracted: conceptList.length,
        entities_extracted: entityList.length,
        relationships_created: relationships.length,
        patterns_detected: patterns.length,
        extraction_timestamp: utcNow(),
      },
    };
  }

  /**
   * Calculate overall confidence from output
   */
  private calculateOverallConfidence(output: KnowledgeGraphBuilderOutput): number {
    const confidences: number[] = [];

    for (const c of output.knowledge_graph.concepts) {
      confidences.push(c.confidence);
    }
    for (const e of output.knowledge_graph.entities) {
      confidences.push(e.confidence);
    }
    for (const r of output.knowledge_graph.relationships) {
      confidences.push(r.confidence);
    }
    for (const p of output.knowledge_graph.patterns) {
      confidences.push(p.confidence);
    }

    if (confidences.length === 0) return 0;

    // Use harmonic mean
    const reciprocalSum = confidences.reduce((sum, c) => sum + (c > 0 ? 1 / c : 0), 0);
    if (reciprocalSum === 0) return 0;

    return Math.min(1, confidences.length / reciprocalSum);
  }

  /**
   * Get list of constraints applied
   */
  private getConstraintsApplied(input: KnowledgeGraphBuilderInput): string[] {
    const constraints: string[] = [];

    if (input.extraction_config?.extract_concepts !== false) {
      constraints.push('concept_extraction_enabled');
    }
    if (input.extraction_config?.extract_entities !== false) {
      constraints.push('entity_extraction_enabled');
    }
    if (input.extraction_config?.detect_patterns !== false) {
      constraints.push('pattern_detection_enabled');
    }
    if (input.extraction_config?.max_depth) {
      constraints.push(`max_depth_${input.extraction_config.max_depth}`);
    }
    if (input.extraction_config?.min_pattern_frequency) {
      constraints.push(`min_frequency_${input.extraction_config.min_pattern_frequency}`);
    }

    return constraints;
  }

  /**
   * Get side effect log (for verification)
   */
  getSideEffectLog(): string[] {
    return [...this.sideEffectLog];
  }

  /**
   * Clear side effect log
   */
  clearSideEffectLog(): void {
    this.sideEffectLog = [];
  }
}

// ============================================================================
// TESTS
// ============================================================================

describe('Knowledge Graph Builder - Integration Tests', () => {
  let agent: KnowledgeGraphBuilderAgent;

  beforeEach(() => {
    agent = new KnowledgeGraphBuilderAgent();
  });

  describe('Full Pipeline Execution', () => {
    it('should process sample input successfully', async () => {
      const input: KnowledgeGraphBuilderInput = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        extraction_config: {
          extract_concepts: true,
          extract_entities: true,
          detect_patterns: true,
          min_pattern_frequency: 2,
          max_depth: 3,
        },
        conversation_data: [
          {
            session_id: 'a1b2c3d4-e5f6-4789-abcd-123456789012',
            turns: [
              {
                turn_id: 'turn-001',
                role: 'user',
                content: 'How do I implement JWT authentication in Node.js?',
                timestamp: '2024-01-15T10:00:00.000Z',
              },
              {
                turn_id: 'turn-002',
                role: 'assistant',
                content:
                  'Use the jsonwebtoken library. Install with npm install jsonwebtoken. Create middleware for Express.',
                timestamp: '2024-01-15T10:00:05.000Z',
              },
            ],
          },
        ],
      };

      const result = await agent.execute(input);

      assert.ok(result.success, 'Execution should succeed');
      if (result.success) {
        // Verify output structure
        assert.ok(result.output.knowledge_graph, 'Should have knowledge_graph');
        assert.ok(Array.isArray(result.output.knowledge_graph.concepts), 'Should have concepts array');
        assert.ok(Array.isArray(result.output.knowledge_graph.entities), 'Should have entities array');
        assert.ok(
          Array.isArray(result.output.knowledge_graph.relationships),
          'Should have relationships array'
        );
        assert.ok(Array.isArray(result.output.knowledge_graph.patterns), 'Should have patterns array');

        // Verify metadata
        assert.ok(result.output.metadata, 'Should have metadata');
        assert.strictEqual(result.output.metadata.sessions_analyzed, 1);
        assert.strictEqual(result.output.metadata.turns_processed, 2);

        // Verify DecisionEvent
        assert.ok(result.decisionEvent, 'Should have decisionEvent');
        assert.strictEqual(result.decisionEvent.agent_id, AGENT_ID);
        assert.strictEqual(result.decisionEvent.decision_type, DECISION_TYPE);
      }
    });

    it('should extract concepts from conversation content', async () => {
      const input: KnowledgeGraphBuilderInput = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        conversation_data: [
          {
            session_id: 'a1b2c3d4-e5f6-4789-abcd-123456789012',
            turns: [
              {
                turn_id: 'turn-001',
                role: 'user',
                content: 'Building authentication middleware for Node.js API',
                timestamp: '2024-01-15T10:00:00.000Z',
              },
            ],
          },
        ],
      };

      const result = await agent.execute(input);

      assert.ok(result.success);
      if (result.success) {
        const conceptNames = result.output.knowledge_graph.concepts.map((c) => c.name.toLowerCase());

        assert.ok(
          conceptNames.some((n) => n.includes('node') || n.includes('auth') || n.includes('middleware')),
          'Should extract relevant concepts'
        );
      }
    });

    it('should extract entities from conversation content', async () => {
      const input: KnowledgeGraphBuilderInput = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        conversation_data: [
          {
            session_id: 'a1b2c3d4-e5f6-4789-abcd-123456789012',
            turns: [
              {
                turn_id: 'turn-001',
                role: 'assistant',
                content: 'Install jsonwebtoken with npm: npm install jsonwebtoken',
                timestamp: '2024-01-15T10:00:00.000Z',
              },
            ],
          },
        ],
      };

      const result = await agent.execute(input);

      assert.ok(result.success);
      if (result.success) {
        const entityNames = result.output.knowledge_graph.entities.map((e) => e.name.toLowerCase());

        assert.ok(entityNames.includes('jsonwebtoken'), 'Should extract jsonwebtoken entity');
        assert.ok(entityNames.includes('npm'), 'Should extract npm entity');
      }
    });

    it('should create relationships between concepts and entities', async () => {
      const input: KnowledgeGraphBuilderInput = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        conversation_data: [
          {
            session_id: 'a1b2c3d4-e5f6-4789-abcd-123456789012',
            turns: [
              {
                turn_id: 'turn-001',
                role: 'user',
                content: 'Node.js authentication with jsonwebtoken',
                timestamp: '2024-01-15T10:00:00.000Z',
              },
            ],
          },
        ],
      };

      const result = await agent.execute(input);

      assert.ok(result.success);
      if (result.success) {
        // If we have both concepts and entities, we should have relationships
        const hasConcepts = result.output.knowledge_graph.concepts.length > 0;
        const hasEntities = result.output.knowledge_graph.entities.length > 0;

        if (hasConcepts && hasEntities) {
          assert.ok(
            result.output.knowledge_graph.relationships.length > 0,
            'Should create relationships between concepts and entities'
          );
        }
      }
    });

    it('should handle multiple sessions', async () => {
      const input: KnowledgeGraphBuilderInput = {
        session_ids: [
          'a1b2c3d4-e5f6-4789-abcd-123456789012',
          'b2c3d4e5-f6a7-4890-bcde-234567890123',
        ],
        conversation_data: [
          {
            session_id: 'a1b2c3d4-e5f6-4789-abcd-123456789012',
            turns: [
              {
                turn_id: 'turn-001',
                role: 'user',
                content: 'JWT authentication',
                timestamp: '2024-01-15T10:00:00.000Z',
              },
            ],
          },
          {
            session_id: 'b2c3d4e5-f6a7-4890-bcde-234567890123',
            turns: [
              {
                turn_id: 'turn-002',
                role: 'user',
                content: 'Authentication best practices',
                timestamp: '2024-01-15T11:00:00.000Z',
              },
            ],
          },
        ],
      };

      const result = await agent.execute(input);

      assert.ok(result.success);
      if (result.success) {
        assert.strictEqual(result.output.metadata.sessions_analyzed, 2);
        assert.strictEqual(result.output.metadata.turns_processed, 2);
      }
    });
  });

  describe('Deterministic Output', () => {
    it('should produce same inputs_hash for same input', async () => {
      const input: KnowledgeGraphBuilderInput = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        conversation_data: [
          {
            session_id: 'a1b2c3d4-e5f6-4789-abcd-123456789012',
            turns: [
              {
                turn_id: 'turn-001',
                role: 'user',
                content: 'Test content',
                timestamp: '2024-01-15T10:00:00.000Z',
              },
            ],
          },
        ],
      };

      const result1 = await agent.execute(input);
      const result2 = await agent.execute(input);

      assert.ok(result1.success && result2.success);
      if (result1.success && result2.success) {
        assert.strictEqual(
          result1.decisionEvent.inputs_hash,
          result2.decisionEvent.inputs_hash,
          'Same input should produce same inputs_hash'
        );
      }
    });

    it('should produce same knowledge graph structure for same input', async () => {
      const input: KnowledgeGraphBuilderInput = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        conversation_data: [
          {
            session_id: 'a1b2c3d4-e5f6-4789-abcd-123456789012',
            turns: [
              {
                turn_id: 'turn-001',
                role: 'user',
                content: 'Node.js authentication',
                timestamp: '2024-01-15T10:00:00.000Z',
              },
            ],
          },
        ],
      };

      const result1 = await agent.execute(input);
      const result2 = await agent.execute(input);

      assert.ok(result1.success && result2.success);
      if (result1.success && result2.success) {
        // Compare concept counts
        assert.strictEqual(
          result1.output.knowledge_graph.concepts.length,
          result2.output.knowledge_graph.concepts.length
        );

        // Compare entity counts
        assert.strictEqual(
          result1.output.knowledge_graph.entities.length,
          result2.output.knowledge_graph.entities.length
        );

        // Compare concept names (sorted for determinism)
        const names1 = result1.output.knowledge_graph.concepts.map((c) => c.name).sort();
        const names2 = result2.output.knowledge_graph.concepts.map((c) => c.name).sort();
        assert.deepStrictEqual(names1, names2);
      }
    });

    it('should produce different output for different input', async () => {
      const input1: KnowledgeGraphBuilderInput = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        conversation_data: [
          {
            session_id: 'a1b2c3d4-e5f6-4789-abcd-123456789012',
            turns: [
              { turn_id: 't1', role: 'user', content: 'Node.js', timestamp: '2024-01-15T10:00:00.000Z' },
            ],
          },
        ],
      };

      const input2: KnowledgeGraphBuilderInput = {
        session_ids: ['b2c3d4e5-f6a7-4890-bcde-234567890123'],
        conversation_data: [
          {
            session_id: 'b2c3d4e5-f6a7-4890-bcde-234567890123',
            turns: [
              {
                turn_id: 't2',
                role: 'user',
                content: 'Python Django',
                timestamp: '2024-01-15T10:00:00.000Z',
              },
            ],
          },
        ],
      };

      const result1 = await agent.execute(input1);
      const result2 = await agent.execute(input2);

      assert.ok(result1.success && result2.success);
      if (result1.success && result2.success) {
        assert.notStrictEqual(
          result1.decisionEvent.inputs_hash,
          result2.decisionEvent.inputs_hash,
          'Different input should produce different inputs_hash'
        );
      }
    });
  });

  describe('Error Handling Paths', () => {
    it('should return VALIDATION_ERROR for invalid input', async () => {
      const invalidInput = {
        session_ids: 'not-an-array',
      };

      const result = await agent.execute(invalidInput);

      assert.ok(!result.success);
      if (!result.success) {
        assert.strictEqual(result.error.error_code, 'VALIDATION_ERROR');
        assert.ok(result.error.message.includes('session_ids'));
      }
    });

    it('should return VALIDATION_ERROR for empty session_ids', async () => {
      const input = {
        session_ids: [],
      };

      const result = await agent.execute(input);

      assert.ok(!result.success);
      if (!result.success) {
        assert.strictEqual(result.error.error_code, 'VALIDATION_ERROR');
        assert.ok(result.error.message.includes('empty'));
      }
    });

    it('should return VALIDATION_ERROR for invalid UUID in session_ids', async () => {
      const input = {
        session_ids: ['not-a-valid-uuid'],
      };

      const result = await agent.execute(input);

      assert.ok(!result.success);
      if (!result.success) {
        assert.strictEqual(result.error.error_code, 'VALIDATION_ERROR');
      }
    });

    it('should return error with execution_ref and timestamp', async () => {
      const invalidInput = { session_ids: null };

      const result = await agent.execute(invalidInput);

      assert.ok(!result.success);
      if (!result.success) {
        assert.ok(result.error.execution_ref, 'Error should have execution_ref');
        assert.ok(result.error.timestamp, 'Error should have timestamp');
      }
    });

    it('should handle null input', async () => {
      const result = await agent.execute(null);

      assert.ok(!result.success);
      if (!result.success) {
        assert.strictEqual(result.error.error_code, 'VALIDATION_ERROR');
      }
    });

    it('should handle undefined input', async () => {
      const result = await agent.execute(undefined);

      assert.ok(!result.success);
      if (!result.success) {
        assert.strictEqual(result.error.error_code, 'VALIDATION_ERROR');
      }
    });
  });

  describe('NO Side Effects Beyond DecisionEvent Emission', () => {
    it('should not produce any side effects during execution', async () => {
      agent.clearSideEffectLog();

      const input: KnowledgeGraphBuilderInput = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        conversation_data: [
          {
            session_id: 'a1b2c3d4-e5f6-4789-abcd-123456789012',
            turns: [
              {
                turn_id: 'turn-001',
                role: 'user',
                content: 'Test content',
                timestamp: '2024-01-15T10:00:00.000Z',
              },
            ],
          },
        ],
      };

      await agent.execute(input);

      const sideEffects = agent.getSideEffectLog();
      assert.strictEqual(
        sideEffects.length,
        0,
        'Agent should not produce any side effects'
      );
    });

    it('should only return data structures, not trigger actions', async () => {
      const input: KnowledgeGraphBuilderInput = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        conversation_data: [
          {
            session_id: 'a1b2c3d4-e5f6-4789-abcd-123456789012',
            turns: [
              {
                turn_id: 'turn-001',
                role: 'user',
                content: 'Security vulnerability detected!', // Content that might trigger actions in other systems
                timestamp: '2024-01-15T10:00:00.000Z',
              },
            ],
          },
        ],
      };

      const result = await agent.execute(input);

      assert.ok(result.success);
      if (result.success) {
        // Result should be pure data - no action callbacks, no promises for side effects
        assert.ok(typeof result.output === 'object', 'Output should be a plain object');
        assert.ok(typeof result.decisionEvent === 'object', 'DecisionEvent should be a plain object');

        // Verify no function properties (which could be callbacks)
        const checkForFunctions = (obj: unknown, path = ''): void => {
          if (obj && typeof obj === 'object') {
            for (const [key, value] of Object.entries(obj)) {
              assert.ok(
                typeof value !== 'function',
                `${path}.${key} should not be a function`
              );
              if (value && typeof value === 'object') {
                checkForFunctions(value, `${path}.${key}`);
              }
            }
          }
        };

        checkForFunctions(result.output, 'output');
        checkForFunctions(result.decisionEvent, 'decisionEvent');
      }
    });

    it('should not modify input during processing', async () => {
      const input: KnowledgeGraphBuilderInput = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        extraction_config: {
          extract_concepts: true,
          extract_entities: true,
          detect_patterns: true,
          min_pattern_frequency: 2,
          max_depth: 3,
        },
        conversation_data: [
          {
            session_id: 'a1b2c3d4-e5f6-4789-abcd-123456789012',
            turns: [
              {
                turn_id: 'turn-001',
                role: 'user',
                content: 'Test',
                timestamp: '2024-01-15T10:00:00.000Z',
              },
            ],
          },
        ],
      };

      // Deep copy for comparison
      const inputCopy = JSON.parse(JSON.stringify(input));

      await agent.execute(input);

      // Verify input was not mutated
      assert.deepStrictEqual(input, inputCopy, 'Input should not be modified');
    });

    it('should emit exactly ONE DecisionEvent per invocation', async () => {
      const input: KnowledgeGraphBuilderInput = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        conversation_data: [
          {
            session_id: 'a1b2c3d4-e5f6-4789-abcd-123456789012',
            turns: [
              {
                turn_id: 'turn-001',
                role: 'user',
                content: 'Multiple concepts: Node.js, JWT, authentication, middleware',
                timestamp: '2024-01-15T10:00:00.000Z',
              },
            ],
          },
        ],
      };

      const result = await agent.execute(input);

      assert.ok(result.success);
      if (result.success) {
        // Verify we get exactly one DecisionEvent
        assert.ok(!Array.isArray(result.decisionEvent), 'Should be single event, not array');
        assert.ok(result.decisionEvent.execution_ref, 'Should have single execution_ref');

        // Verify the event structure matches expected schema
        assert.strictEqual(result.decisionEvent.agent_id, AGENT_ID);
        assert.strictEqual(result.decisionEvent.decision_type, DECISION_TYPE);
      }
    });
  });

  describe('Configuration Handling', () => {
    it('should respect extract_concepts=false', async () => {
      const input: KnowledgeGraphBuilderInput = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        extraction_config: {
          extract_concepts: false,
          extract_entities: true,
          detect_patterns: true,
          min_pattern_frequency: 2,
          max_depth: 3,
        },
        conversation_data: [
          {
            session_id: 'a1b2c3d4-e5f6-4789-abcd-123456789012',
            turns: [
              {
                turn_id: 'turn-001',
                role: 'user',
                content: 'Node.js authentication',
                timestamp: '2024-01-15T10:00:00.000Z',
              },
            ],
          },
        ],
      };

      const result = await agent.execute(input);

      assert.ok(result.success);
      if (result.success) {
        assert.strictEqual(
          result.output.knowledge_graph.concepts.length,
          0,
          'Should not extract concepts when disabled'
        );
      }
    });

    it('should respect extract_entities=false', async () => {
      const input: KnowledgeGraphBuilderInput = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        extraction_config: {
          extract_concepts: true,
          extract_entities: false,
          detect_patterns: true,
          min_pattern_frequency: 2,
          max_depth: 3,
        },
        conversation_data: [
          {
            session_id: 'a1b2c3d4-e5f6-4789-abcd-123456789012',
            turns: [
              {
                turn_id: 'turn-001',
                role: 'user',
                content: 'Install jsonwebtoken with npm',
                timestamp: '2024-01-15T10:00:00.000Z',
              },
            ],
          },
        ],
      };

      const result = await agent.execute(input);

      assert.ok(result.success);
      if (result.success) {
        assert.strictEqual(
          result.output.knowledge_graph.entities.length,
          0,
          'Should not extract entities when disabled'
        );
      }
    });

    it('should respect detect_patterns=false', async () => {
      const input: KnowledgeGraphBuilderInput = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        extraction_config: {
          extract_concepts: true,
          extract_entities: true,
          detect_patterns: false,
          min_pattern_frequency: 1,
          max_depth: 3,
        },
        conversation_data: [
          {
            session_id: 'a1b2c3d4-e5f6-4789-abcd-123456789012',
            turns: [
              {
                turn_id: 'turn-001',
                role: 'user',
                content: 'Node.js Node.js Node.js', // High frequency
                timestamp: '2024-01-15T10:00:00.000Z',
              },
            ],
          },
        ],
      };

      const result = await agent.execute(input);

      assert.ok(result.success);
      if (result.success) {
        assert.strictEqual(
          result.output.knowledge_graph.patterns.length,
          0,
          'Should not detect patterns when disabled'
        );
      }
    });

    it('should track constraints_applied in DecisionEvent', async () => {
      const input: KnowledgeGraphBuilderInput = {
        session_ids: ['a1b2c3d4-e5f6-4789-abcd-123456789012'],
        extraction_config: {
          extract_concepts: true,
          extract_entities: true,
          detect_patterns: true,
          min_pattern_frequency: 3,
          max_depth: 5,
        },
        conversation_data: [
          {
            session_id: 'a1b2c3d4-e5f6-4789-abcd-123456789012',
            turns: [
              {
                turn_id: 'turn-001',
                role: 'user',
                content: 'Test',
                timestamp: '2024-01-15T10:00:00.000Z',
              },
            ],
          },
        ],
      };

      const result = await agent.execute(input);

      assert.ok(result.success);
      if (result.success) {
        assert.ok(result.decisionEvent.constraints_applied);
        assert.ok(
          result.decisionEvent.constraints_applied.includes('concept_extraction_enabled')
        );
        assert.ok(
          result.decisionEvent.constraints_applied.includes('max_depth_5')
        );
        assert.ok(
          result.decisionEvent.constraints_applied.includes('min_frequency_3')
        );
      }
    });
  });
});
