/**
 * Tests for artifact builder
 *
 * Verifies artifact extraction for each agent type, stable URI generation,
 * and content hash consistency.
 */

import { describe, it, expect } from 'vitest';
import { buildArtifacts } from '../artifact-builder.js';
import { ArtifactReferenceSchema } from '../types.js';

// ============================================================================
// CONVERSATION MEMORY AGENT
// ============================================================================

describe('buildArtifacts - conversation-memory-agent', () => {
  it('should extract nodes_created and edges_created', async () => {
    const output = {
      conversation_id: crypto.randomUUID(),
      session_id: crypto.randomUUID(),
      nodes_created: [
        { node_id: crypto.randomUUID(), node_type: 'prompt', turn_index: 0 },
        { node_id: crypto.randomUUID(), node_type: 'response', turn_index: 0 },
      ],
      edges_created: [
        {
          edge_id: crypto.randomUUID(),
          edge_type: 'responds_to',
          from_node_id: crypto.randomUUID(),
          to_node_id: crypto.randomUUID(),
        },
      ],
      capture_timestamp: new Date().toISOString(),
      turn_count: 1,
    };

    const executionId = crypto.randomUUID();
    const artifacts = await buildArtifacts('conversation-memory-agent', executionId, output);

    expect(artifacts).toHaveLength(2);
    expect(artifacts[0].artifact_type).toBe('nodes_created');
    expect(artifacts[1].artifact_type).toBe('edges_created');
  });

  it('should skip empty arrays', async () => {
    const output = {
      nodes_created: [],
      edges_created: [],
    };

    const artifacts = await buildArtifacts('conversation-memory-agent', crypto.randomUUID(), output);
    expect(artifacts).toHaveLength(0);
  });
});

// ============================================================================
// LONG TERM PATTERN AGENT
// ============================================================================

describe('buildArtifacts - long-term-pattern-agent', () => {
  it('should extract patterns and statistics', async () => {
    const output = {
      analysis_id: crypto.randomUUID(),
      patterns: [
        {
          pattern_id: crypto.randomUUID(),
          pattern_type: 'topic_recurrence',
          pattern_signature: 'test',
          occurrence_count: 5,
          confidence: 0.9,
          relevance_score: 0.8,
          first_seen: new Date().toISOString(),
          last_seen: new Date().toISOString(),
        },
      ],
      statistics: {
        sessions_analyzed: 10,
        nodes_scanned: 100,
        patterns_found: 1,
      },
      time_range_analyzed: {
        from_timestamp: new Date().toISOString(),
        to_timestamp: new Date().toISOString(),
      },
      analysis_timestamp: new Date().toISOString(),
    };

    const artifacts = await buildArtifacts('long-term-pattern-agent', crypto.randomUUID(), output);
    expect(artifacts).toHaveLength(2);
    expect(artifacts[0].artifact_type).toBe('detected_patterns');
    expect(artifacts[1].artifact_type).toBe('analysis_statistics');
  });
});

// ============================================================================
// KNOWLEDGE GRAPH BUILDER AGENT
// ============================================================================

describe('buildArtifacts - knowledge-graph-builder-agent', () => {
  it('should extract concepts, relationships, and patterns', async () => {
    const output = {
      request_id: crypto.randomUUID(),
      concepts: [
        { concept_id: crypto.randomUUID(), name: 'Test', normalized_name: 'test', type: 'entity' },
      ],
      relationships: [
        {
          relationship_id: crypto.randomUUID(),
          source_concept_id: crypto.randomUUID(),
          target_concept_id: crypto.randomUUID(),
          relationship_type: 'related_to',
        },
      ],
      patterns: [
        {
          pattern_id: crypto.randomUUID(),
          pattern_type: 'recurring_theme',
          description: 'test',
          involved_concepts: [],
          occurrences: 3,
          confidence: 0.8,
        },
      ],
      statistics: { total_concepts: 1, total_relationships: 1, total_patterns: 1 },
    };

    const artifacts = await buildArtifacts('knowledge-graph-builder-agent', crypto.randomUUID(), output);
    expect(artifacts).toHaveLength(3);
    expect(artifacts.map((a) => a.artifact_type).sort()).toEqual([
      'detected_patterns',
      'extracted_concepts',
      'extracted_relationships',
    ]);
  });
});

// ============================================================================
// MEMORY RETRIEVAL AGENT
// ============================================================================

describe('buildArtifacts - memory-retrieval-agent', () => {
  it('should extract subgraph and stats', async () => {
    const output = {
      query_id: crypto.randomUUID(),
      query_type: 'subgraph',
      subgraph: {
        nodes: [{ node_id: crypto.randomUUID(), node_type: 'prompt' }],
        edges: [],
      },
      traversal_stats: {
        max_depth_reached: 2,
        nodes_visited: 10,
        edges_traversed: 8,
      },
      total_nodes_retrieved: 1,
      total_edges_retrieved: 0,
    };

    const artifacts = await buildArtifacts('memory-retrieval-agent', crypto.randomUUID(), output);
    expect(artifacts).toHaveLength(2);
    expect(artifacts[0].artifact_type).toBe('retrieved_subgraph');
    expect(artifacts[1].artifact_type).toBe('retrieval_statistics');
  });
});

// ============================================================================
// STABLE URI FORMAT
// ============================================================================

describe('Stable artifact URIs', () => {
  it('should follow the memory-graph/{agent}/{exec_id}/{type}/{index} format', async () => {
    const executionId = crypto.randomUUID();
    const output = {
      nodes_created: [{ node_id: crypto.randomUUID(), node_type: 'prompt' }],
    };

    const artifacts = await buildArtifacts('conversation-memory-agent', executionId, output);
    expect(artifacts[0].artifact_ref).toBe(
      `memory-graph/conversation-memory-agent/${executionId}/nodes_created/0`,
    );
  });
});

// ============================================================================
// CONTENT HASH CONSISTENCY
// ============================================================================

describe('Content hash', () => {
  it('should produce consistent hash for same content', async () => {
    const output = {
      nodes_created: [{ node_id: 'fixed-id', node_type: 'prompt' }],
    };

    const artifacts1 = await buildArtifacts('conversation-memory-agent', 'exec-1', output);
    const artifacts2 = await buildArtifacts('conversation-memory-agent', 'exec-2', output);

    expect(artifacts1[0].content_hash).toBe(artifacts2[0].content_hash);
    expect(artifacts1[0].content_hash).toHaveLength(64);
  });

  it('should produce different hash for different content', async () => {
    const output1 = { nodes_created: [{ node_id: 'id-1', node_type: 'prompt' }] };
    const output2 = { nodes_created: [{ node_id: 'id-2', node_type: 'prompt' }] };

    const artifacts1 = await buildArtifacts('conversation-memory-agent', 'exec-1', output1);
    const artifacts2 = await buildArtifacts('conversation-memory-agent', 'exec-1', output2);

    expect(artifacts1[0].content_hash).not.toBe(artifacts2[0].content_hash);
  });
});

// ============================================================================
// SCHEMA VALIDATION
// ============================================================================

describe('Schema compliance', () => {
  it('all artifacts should pass ArtifactReferenceSchema validation', async () => {
    const output = {
      nodes_created: [{ node_id: crypto.randomUUID(), node_type: 'prompt' }],
      edges_created: [
        {
          edge_id: crypto.randomUUID(),
          edge_type: 'follows',
          from_node_id: crypto.randomUUID(),
          to_node_id: crypto.randomUUID(),
        },
      ],
    };

    const artifacts = await buildArtifacts('conversation-memory-agent', crypto.randomUUID(), output);

    for (const artifact of artifacts) {
      const result = ArtifactReferenceSchema.safeParse(artifact);
      expect(result.success).toBe(true);
    }
  });
});

// ============================================================================
// GENERIC EXTRACTION
// ============================================================================

describe('Generic extraction for unknown agents', () => {
  it('should extract non-empty arrays and objects', async () => {
    const output = {
      results: [1, 2, 3],
      summary: { total: 3 },
      empty_array: [],
      null_field: null,
    };

    const artifacts = await buildArtifacts('unknown-agent', crypto.randomUUID(), output);
    expect(artifacts).toHaveLength(2); // results + summary
    expect(artifacts.map((a) => a.artifact_type).sort()).toEqual(['results', 'summary']);
  });

  it('should return empty for null/undefined output', async () => {
    const artifacts = await buildArtifacts('unknown-agent', crypto.randomUUID(), null);
    expect(artifacts).toHaveLength(0);
  });
});
