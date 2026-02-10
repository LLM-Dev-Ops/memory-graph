/**
 * Artifact Builder
 *
 * Extracts artifacts from agent outputs and creates stable ArtifactReference
 * objects with content-addressable hashes and URIs.
 *
 * Each agent type produces a known output shape. This module maps those outputs
 * to ArtifactReference instances attached to the agent-level span.
 */

import type { ArtifactReference } from './types.js';
import { REPO_NAME } from './types.js';

/**
 * Compute SHA-256 hash of content for artifact verification.
 */
async function sha256(content: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(content);
  const hashBuffer = await crypto.subtle.digest('SHA-256', data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  return hashArray.map((b) => b.toString(16).padStart(2, '0')).join('');
}

/**
 * Build a single artifact reference from a content payload.
 */
async function buildArtifact(
  agentId: string,
  executionId: string,
  artifactType: string,
  index: number,
  content: unknown,
): Promise<ArtifactReference> {
  const serialized = JSON.stringify(content);
  const contentHash = await sha256(serialized);

  return {
    artifact_ref: `${REPO_NAME}/${agentId}/${executionId}/${artifactType}/${index}`,
    artifact_type: artifactType,
    content_hash: contentHash,
    size_bytes: new TextEncoder().encode(serialized).byteLength,
    content,
  };
}

/**
 * Extract artifacts from a conversation-memory agent output.
 *
 * Output shape: { nodes_created: NodeReference[], edges_created: EdgeReference[], ... }
 */
async function extractConversationMemoryArtifacts(
  executionId: string,
  output: Record<string, unknown>,
): Promise<ArtifactReference[]> {
  const artifacts: ArtifactReference[] = [];
  const agentId = 'conversation-memory-agent';

  if (output.nodes_created && Array.isArray(output.nodes_created) && output.nodes_created.length > 0) {
    artifacts.push(await buildArtifact(agentId, executionId, 'nodes_created', 0, output.nodes_created));
  }
  if (output.edges_created && Array.isArray(output.edges_created) && output.edges_created.length > 0) {
    artifacts.push(await buildArtifact(agentId, executionId, 'edges_created', 0, output.edges_created));
  }

  return artifacts;
}

/**
 * Extract artifacts from a long-term-pattern agent output.
 *
 * Output shape: { patterns: DetectedPattern[], statistics: AnalysisStatistics, ... }
 */
async function extractLongTermPatternArtifacts(
  executionId: string,
  output: Record<string, unknown>,
): Promise<ArtifactReference[]> {
  const artifacts: ArtifactReference[] = [];
  const agentId = 'long-term-pattern-agent';

  if (output.patterns && Array.isArray(output.patterns) && output.patterns.length > 0) {
    artifacts.push(await buildArtifact(agentId, executionId, 'detected_patterns', 0, output.patterns));
  }
  if (output.statistics) {
    artifacts.push(await buildArtifact(agentId, executionId, 'analysis_statistics', 0, output.statistics));
  }

  return artifacts;
}

/**
 * Extract artifacts from a knowledge-graph-builder agent output.
 *
 * Output shape: { concepts: ExtractedConcept[], relationships: ExtractedRelationship[], patterns: DetectedPattern[], ... }
 */
async function extractKnowledgeGraphBuilderArtifacts(
  executionId: string,
  output: Record<string, unknown>,
): Promise<ArtifactReference[]> {
  const artifacts: ArtifactReference[] = [];
  const agentId = 'knowledge-graph-builder-agent';

  if (output.concepts && Array.isArray(output.concepts) && output.concepts.length > 0) {
    artifacts.push(await buildArtifact(agentId, executionId, 'extracted_concepts', 0, output.concepts));
  }
  if (output.relationships && Array.isArray(output.relationships) && output.relationships.length > 0) {
    artifacts.push(await buildArtifact(agentId, executionId, 'extracted_relationships', 0, output.relationships));
  }
  if (output.patterns && Array.isArray(output.patterns) && output.patterns.length > 0) {
    artifacts.push(await buildArtifact(agentId, executionId, 'detected_patterns', 0, output.patterns));
  }

  return artifacts;
}

/**
 * Extract artifacts from a memory-retrieval agent output.
 *
 * Output shape: { subgraph: RetrievedSubgraph, traversal_stats: TraversalStats, ... }
 */
async function extractMemoryRetrievalArtifacts(
  executionId: string,
  output: Record<string, unknown>,
): Promise<ArtifactReference[]> {
  const artifacts: ArtifactReference[] = [];
  const agentId = 'memory-retrieval-agent';

  if (output.subgraph) {
    artifacts.push(await buildArtifact(agentId, executionId, 'retrieved_subgraph', 0, output.subgraph));
  }
  if (output.traversal_stats) {
    artifacts.push(await buildArtifact(agentId, executionId, 'retrieval_statistics', 0, output.traversal_stats));
  }

  return artifacts;
}

/**
 * Extract artifacts from a decision-memory agent output.
 *
 * Output shape: { nodes_created: GraphNodeCreated[], edges_created: GraphEdgeCreated[], ... }
 */
async function extractDecisionMemoryArtifacts(
  executionId: string,
  output: Record<string, unknown>,
): Promise<ArtifactReference[]> {
  const artifacts: ArtifactReference[] = [];
  const agentId = 'decision-memory-agent';

  if (output.nodes_created && Array.isArray(output.nodes_created) && output.nodes_created.length > 0) {
    artifacts.push(await buildArtifact(agentId, executionId, 'decision_nodes', 0, output.nodes_created));
  }
  if (output.edges_created && Array.isArray(output.edges_created) && output.edges_created.length > 0) {
    artifacts.push(await buildArtifact(agentId, executionId, 'decision_edges', 0, output.edges_created));
  }

  return artifacts;
}

/**
 * Extract artifacts from a prompt-lineage agent output.
 */
async function extractPromptLineageArtifacts(
  executionId: string,
  output: Record<string, unknown>,
): Promise<ArtifactReference[]> {
  const artifacts: ArtifactReference[] = [];
  const agentId = 'prompt-lineage-agent';

  if (output.nodes_created && Array.isArray(output.nodes_created) && output.nodes_created.length > 0) {
    artifacts.push(await buildArtifact(agentId, executionId, 'lineage_nodes', 0, output.nodes_created));
  }
  if (output.edges_created && Array.isArray(output.edges_created) && output.edges_created.length > 0) {
    artifacts.push(await buildArtifact(agentId, executionId, 'lineage_edges', 0, output.edges_created));
  }

  return artifacts;
}

/**
 * Build artifacts for an agent's output. Dispatches to the appropriate extractor
 * based on agent_id.
 */
export async function buildArtifacts(
  agentId: string,
  executionId: string,
  agentOutput: unknown,
): Promise<ArtifactReference[]> {
  if (!agentOutput || typeof agentOutput !== 'object') {
    return [];
  }

  const output = agentOutput as Record<string, unknown>;

  switch (agentId) {
    case 'conversation-memory-agent':
      return extractConversationMemoryArtifacts(executionId, output);
    case 'long-term-pattern-agent':
      return extractLongTermPatternArtifacts(executionId, output);
    case 'knowledge-graph-builder-agent':
      return extractKnowledgeGraphBuilderArtifacts(executionId, output);
    case 'memory-retrieval-agent':
      return extractMemoryRetrievalArtifacts(executionId, output);
    case 'decision-memory-agent':
      return extractDecisionMemoryArtifacts(executionId, output);
    case 'prompt-lineage-agent':
      return extractPromptLineageArtifacts(executionId, output);
    default:
      // Unknown agent -- attempt generic extraction of any array/object fields
      return extractGenericArtifacts(agentId, executionId, output);
  }
}

/**
 * Generic artifact extraction for unknown agent types. Creates one artifact
 * per top-level field that is a non-empty array or non-null object.
 */
async function extractGenericArtifacts(
  agentId: string,
  executionId: string,
  output: Record<string, unknown>,
): Promise<ArtifactReference[]> {
  const artifacts: ArtifactReference[] = [];
  let index = 0;

  for (const [key, value] of Object.entries(output)) {
    if (value === null || value === undefined) continue;

    if (Array.isArray(value) && value.length > 0) {
      artifacts.push(await buildArtifact(agentId, executionId, key, index++, value));
    } else if (typeof value === 'object' && !Array.isArray(value)) {
      artifacts.push(await buildArtifact(agentId, executionId, key, index++, value));
    }
  }

  return artifacts;
}
