/**
 * Knowledge Graph Builder Module
 *
 * Constructs knowledge graphs from extracted concepts and relationships.
 * This module handles graph construction, merging, and pattern detection.
 *
 * EXPLICIT NON-RESPONSIBILITIES:
 * - This module does NOT persist data
 * - This module does NOT enforce policies
 * - This module does NOT trigger actions
 */

import {
  type TextContent,
  type GraphOptions,
  type ExtractedConcept,
  type ExtractedRelationship,
  type DetectedPattern,
  type GraphStatistics,
  type KnowledgeGraphBuilderOutput,
} from './types.js';
import { KnowledgeExtractor, createExtractor } from './extractor.js';
import { ConfidenceCalculator, createConfidenceCalculator } from './confidence.js';

/**
 * Internal graph representation
 */
interface InternalGraph {
  concepts: Map<string, ExtractedConcept>;
  relationships: Map<string, ExtractedRelationship>;
  adjacencyList: Map<string, Set<string>>;
  reverseAdjacencyList: Map<string, Set<string>>;
}

// Note: PatternContext interface reserved for future pattern detection enhancements

/**
 * Knowledge Graph Builder
 *
 * Builds knowledge graphs from text content using extraction and
 * confidence scoring.
 */
export class KnowledgeGraphBuilder {
  private readonly extractor: KnowledgeExtractor;
  private readonly confidenceCalculator: ConfidenceCalculator;
  private readonly options: Required<GraphOptions>;

  constructor(
    extractionOptions?: Parameters<typeof createExtractor>[0],
    graphOptions?: Partial<GraphOptions>,
    minConfidence?: number
  ) {
    this.extractor = createExtractor(extractionOptions);
    this.confidenceCalculator = createConfidenceCalculator(minConfidence);
    this.options = {
      merge_similar_concepts: graphOptions?.merge_similar_concepts ?? true,
      similarity_threshold: graphOptions?.similarity_threshold ?? 0.8,
      create_temporal_edges: graphOptions?.create_temporal_edges ?? true,
      compute_centrality: graphOptions?.compute_centrality ?? false,
      max_relationship_depth: graphOptions?.max_relationship_depth ?? 3,
    };
  }

  /**
   * Build a knowledge graph from text contents
   */
  build(
    requestId: string,
    texts: TextContent[],
    sessionId?: string,
    conversationId?: string
  ): KnowledgeGraphBuilderOutput {
    const extractionStart = Date.now();

    // Step 1: Extract raw knowledge
    const rawExtraction = this.extractor.extract(texts);

    // Step 2: Finalize concepts
    let concepts = this.extractor.finalizeConcepts(rawExtraction);

    // Step 3: Merge similar concepts if enabled
    if (this.options.merge_similar_concepts) {
      concepts = this.mergeSimilarConcepts(concepts);
    }

    // Step 4: Create concept ID map
    const conceptMap = new Map<string, string>();
    for (const concept of concepts) {
      conceptMap.set(concept.normalized_name, concept.concept_id);
    }

    // Step 5: Finalize relationships
    let relationships = this.extractor.finalizeRelationships(rawExtraction, conceptMap);

    const extractionDuration = Date.now() - extractionStart;
    const graphBuildStart = Date.now();

    // Step 6: Build internal graph
    const graph = this.buildInternalGraph(concepts, relationships);

    // Step 7: Add temporal edges if enabled
    if (this.options.create_temporal_edges) {
      const temporalEdges = this.createTemporalEdges(texts, concepts);
      relationships = [...relationships, ...temporalEdges];
      this.addRelationshipsToGraph(graph, temporalEdges);
    }

    // Step 8: Update confidence scores with graph context
    this.updateConfidenceScores(concepts, relationships, texts.length);

    // Step 9: Detect patterns
    const patterns = this.detectPatterns(graph, concepts, texts);

    // Step 10: Calculate statistics
    // Pre-filter statistics computed but not needed for final output
    // (we recalculate after filtering)
    const graphBuildDuration = Date.now() - graphBuildStart;

    // Step 11: Filter by confidence
    const filteredConcepts = this.confidenceCalculator.filterByConfidence(concepts);
    const filteredRelationships = this.confidenceCalculator.filterByConfidence(relationships);
    const filteredPatterns = this.confidenceCalculator.filterByConfidence(patterns);

    return {
      request_id: requestId,
      session_id: sessionId,
      conversation_id: conversationId,
      concepts: filteredConcepts,
      relationships: filteredRelationships,
      patterns: filteredPatterns,
      statistics: this.calculateStatistics(filteredConcepts, filteredRelationships, filteredPatterns, graph),
      build_timestamp: new Date().toISOString(),
      processing_metadata: {
        texts_processed: texts.length,
        total_characters: texts.reduce((sum, t) => sum + t.text.length, 0),
        extraction_duration_ms: extractionDuration,
        graph_build_duration_ms: graphBuildDuration,
      },
    };
  }

  /**
   * Merge similar concepts based on normalized names
   */
  private mergeSimilarConcepts(concepts: ExtractedConcept[]): ExtractedConcept[] {
    const merged = new Map<string, ExtractedConcept>();

    for (const concept of concepts) {
      let foundSimilar = false;

      for (const [existingName, existing] of merged) {
        if (this.calculateSimilarity(concept.normalized_name, existingName) >= this.options.similarity_threshold) {
          // Merge into existing concept
          existing.frequency += concept.frequency;
          existing.source_content_ids = [
            ...new Set([...existing.source_content_ids, ...concept.source_content_ids]),
          ];
          existing.context_snippets = [
            ...(existing.context_snippets ?? []),
            ...(concept.context_snippets ?? []),
          ].slice(0, 10);
          existing.confidence = Math.max(existing.confidence, concept.confidence);
          foundSimilar = true;
          break;
        }
      }

      if (!foundSimilar) {
        merged.set(concept.normalized_name, { ...concept });
      }
    }

    return [...merged.values()];
  }

  /**
   * Calculate similarity between two strings using Jaccard similarity
   */
  private calculateSimilarity(str1: string, str2: string): number {
    const set1 = new Set(str1.toLowerCase().split(/\s+/));
    const set2 = new Set(str2.toLowerCase().split(/\s+/));

    const intersection = new Set([...set1].filter(x => set2.has(x)));
    const union = new Set([...set1, ...set2]);

    return union.size > 0 ? intersection.size / union.size : 0;
  }

  /**
   * Build internal graph representation
   */
  private buildInternalGraph(
    concepts: ExtractedConcept[],
    relationships: ExtractedRelationship[]
  ): InternalGraph {
    const graph: InternalGraph = {
      concepts: new Map(),
      relationships: new Map(),
      adjacencyList: new Map(),
      reverseAdjacencyList: new Map(),
    };

    // Add concepts
    for (const concept of concepts) {
      graph.concepts.set(concept.concept_id, concept);
      graph.adjacencyList.set(concept.concept_id, new Set());
      graph.reverseAdjacencyList.set(concept.concept_id, new Set());
    }

    // Add relationships
    this.addRelationshipsToGraph(graph, relationships);

    return graph;
  }

  /**
   * Add relationships to graph
   */
  private addRelationshipsToGraph(
    graph: InternalGraph,
    relationships: ExtractedRelationship[]
  ): void {
    for (const rel of relationships) {
      graph.relationships.set(rel.relationship_id, rel);

      const adjacency = graph.adjacencyList.get(rel.source_concept_id);
      if (adjacency) {
        adjacency.add(rel.target_concept_id);
      }

      const reverseAdjacency = graph.reverseAdjacencyList.get(rel.target_concept_id);
      if (reverseAdjacency) {
        reverseAdjacency.add(rel.source_concept_id);
      }
    }
  }

  /**
   * Create temporal edges based on text timestamps
   */
  private createTemporalEdges(
    texts: TextContent[],
    concepts: ExtractedConcept[]
  ): ExtractedRelationship[] {
    const temporalEdges: ExtractedRelationship[] = [];

    // Sort texts by timestamp
    const sortedTexts = [...texts]
      .filter(t => t.timestamp)
      .sort((a, b) => new Date(a.timestamp!).getTime() - new Date(b.timestamp!).getTime());

    if (sortedTexts.length < 2) return temporalEdges;

    // Find concepts that appear in consecutive texts
    for (let i = 0; i < sortedTexts.length - 1; i++) {
      const currentText = sortedTexts[i]!;
      const nextText = sortedTexts[i + 1]!;

      const currentConcepts = concepts.filter(c =>
        c.source_content_ids.includes(currentText.content_id)
      );
      const nextConcepts = concepts.filter(c =>
        c.source_content_ids.includes(nextText.content_id)
      );

      // Create temporal edges for concept transitions
      for (const current of currentConcepts) {
        for (const next of nextConcepts) {
          if (current.concept_id !== next.concept_id) {
            temporalEdges.push({
              relationship_id: crypto.randomUUID(),
              source_concept_id: current.concept_id,
              target_concept_id: next.concept_id,
              relationship_type: 'precedes',
              confidence: 0.6,
              weight: 1,
              evidence: [{
                content_id: currentText.content_id,
                snippet: `Temporal: ${current.name} -> ${next.name}`,
              }],
            });
          }
        }
      }
    }

    return temporalEdges;
  }

  /**
   * Update confidence scores based on graph structure
   */
  private updateConfidenceScores(
    concepts: ExtractedConcept[],
    relationships: ExtractedRelationship[],
    totalTexts: number
  ): void {
    // Update concept confidences
    for (const concept of concepts) {
      const result = this.confidenceCalculator.calculateConceptConfidence(
        concept,
        totalTexts,
        concepts
      );
      concept.confidence = result.score;
    }

    // Create concept lookup
    const conceptLookup = new Map<string, ExtractedConcept>();
    for (const concept of concepts) {
      conceptLookup.set(concept.concept_id, concept);
    }

    // Update relationship confidences
    for (const rel of relationships) {
      const source = conceptLookup.get(rel.source_concept_id);
      const target = conceptLookup.get(rel.target_concept_id);

      if (source && target) {
        const result = this.confidenceCalculator.calculateRelationshipConfidence(
          rel,
          source,
          target,
          totalTexts
        );
        rel.confidence = result.score;
      }
    }
  }

  /**
   * Detect patterns in the knowledge graph
   */
  private detectPatterns(
    graph: InternalGraph,
    concepts: ExtractedConcept[],
    texts: TextContent[]
  ): DetectedPattern[] {
    const patterns: DetectedPattern[] = [];

    // Detect recurring themes
    patterns.push(...this.detectRecurringThemes(concepts, texts));

    // Detect temporal sequences
    patterns.push(...this.detectTemporalSequences(graph, texts));

    // Detect co-occurrence patterns
    patterns.push(...this.detectCooccurrencePatterns(concepts));

    // Detect structural patterns (hubs, clusters)
    patterns.push(...this.detectStructuralPatterns(graph, concepts));

    return patterns;
  }

  /**
   * Detect recurring themes
   */
  private detectRecurringThemes(
    concepts: ExtractedConcept[],
    texts: TextContent[]
  ): DetectedPattern[] {
    const patterns: DetectedPattern[] = [];
    const threshold = Math.max(2, Math.floor(texts.length * 0.3));

    // Find concepts that appear frequently
    const frequentConcepts = concepts.filter(c => c.frequency >= threshold);

    if (frequentConcepts.length >= 2) {
      patterns.push({
        pattern_id: crypto.randomUUID(),
        pattern_type: 'recurring_theme',
        description: `Recurring themes: ${frequentConcepts.slice(0, 5).map(c => c.name).join(', ')}`,
        involved_concepts: frequentConcepts.slice(0, 10).map(c => c.concept_id),
        occurrences: frequentConcepts.length,
        confidence: Math.min(frequentConcepts.length / concepts.length + 0.3, 1.0),
      });
    }

    return patterns;
  }

  /**
   * Detect temporal sequences
   */
  private detectTemporalSequences(
    graph: InternalGraph,
    _texts: TextContent[]
  ): DetectedPattern[] {
    const patterns: DetectedPattern[] = [];

    // Find chains of 'precedes' relationships
    const precedesRels = [...graph.relationships.values()]
      .filter(r => r.relationship_type === 'precedes');

    if (precedesRels.length < 2) return patterns;

    // Build sequence chains
    const chains = this.findChains(precedesRels, 3);

    for (const chain of chains) {
      if (chain.length >= 3) {
        patterns.push({
          pattern_id: crypto.randomUUID(),
          pattern_type: 'temporal_sequence',
          description: `Temporal sequence of ${chain.length} concepts`,
          involved_concepts: chain,
          occurrences: 1,
          confidence: Math.min(0.5 + chain.length * 0.1, 0.9),
        });
      }
    }

    return patterns;
  }

  /**
   * Find chains of concepts through relationships
   */
  private findChains(
    relationships: ExtractedRelationship[],
    maxDepth: number
  ): string[][] {
    const chains: string[][] = [];
    const visited = new Set<string>();

    // Build adjacency from relationships
    const adjacency = new Map<string, string[]>();
    for (const rel of relationships) {
      const targets = adjacency.get(rel.source_concept_id) ?? [];
      targets.push(rel.target_concept_id);
      adjacency.set(rel.source_concept_id, targets);
    }

    // Find starting points (concepts with no incoming edges)
    const hasIncoming = new Set(relationships.map(r => r.target_concept_id));
    const startingPoints = [...adjacency.keys()].filter(id => !hasIncoming.has(id));

    // DFS to find chains
    for (const start of startingPoints) {
      const chain: string[] = [start];
      this.dfsChain(start, adjacency, chain, maxDepth, chains, visited);
    }

    return chains;
  }

  /**
   * DFS helper for chain finding
   */
  private dfsChain(
    current: string,
    adjacency: Map<string, string[]>,
    chain: string[],
    maxDepth: number,
    chains: string[][],
    visited: Set<string>
  ): void {
    if (chain.length >= maxDepth || visited.has(current)) {
      if (chain.length >= 3) {
        chains.push([...chain]);
      }
      return;
    }

    visited.add(current);
    const neighbors = adjacency.get(current) ?? [];

    if (neighbors.length === 0 && chain.length >= 3) {
      chains.push([...chain]);
    }

    for (const neighbor of neighbors) {
      chain.push(neighbor);
      this.dfsChain(neighbor, adjacency, chain, maxDepth, chains, visited);
      chain.pop();
    }

    visited.delete(current);
  }

  /**
   * Detect co-occurrence patterns
   */
  private detectCooccurrencePatterns(concepts: ExtractedConcept[]): DetectedPattern[] {
    const patterns: DetectedPattern[] = [];

    // Build co-occurrence matrix
    const cooccurrence = new Map<string, Map<string, number>>();

    for (const concept of concepts) {
      cooccurrence.set(concept.concept_id, new Map());
    }

    for (let i = 0; i < concepts.length; i++) {
      for (let j = i + 1; j < concepts.length; j++) {
        const c1 = concepts[i]!;
        const c2 = concepts[j]!;

        const shared = c1.source_content_ids.filter(
          id => c2.source_content_ids.includes(id)
        ).length;

        if (shared > 0) {
          cooccurrence.get(c1.concept_id)?.set(c2.concept_id, shared);
          cooccurrence.get(c2.concept_id)?.set(c1.concept_id, shared);
        }
      }
    }

    // Find strong co-occurrences
    const threshold = 2;
    const strongPairs: Array<[string, string, number]> = [];

    for (const [id1, neighbors] of cooccurrence) {
      for (const [id2, count] of neighbors) {
        if (count >= threshold && id1 < id2) {
          strongPairs.push([id1, id2, count]);
        }
      }
    }

    if (strongPairs.length > 0) {
      // Group into clusters
      const clusters = this.clusterCooccurrences(strongPairs);

      for (const cluster of clusters) {
        if (cluster.length >= 2) {
          patterns.push({
            pattern_id: crypto.randomUUID(),
            pattern_type: 'co_occurrence',
            description: `Co-occurring concept cluster of ${cluster.length} concepts`,
            involved_concepts: cluster,
            occurrences: cluster.length,
            confidence: Math.min(0.5 + cluster.length * 0.08, 0.95),
          });
        }
      }
    }

    return patterns;
  }

  /**
   * Cluster co-occurring concepts using union-find
   */
  private clusterCooccurrences(pairs: Array<[string, string, number]>): string[][] {
    const parent = new Map<string, string>();

    const find = (x: string): string => {
      if (!parent.has(x)) parent.set(x, x);
      if (parent.get(x) !== x) {
        parent.set(x, find(parent.get(x)!));
      }
      return parent.get(x)!;
    };

    const union = (x: string, y: string): void => {
      const px = find(x);
      const py = find(y);
      if (px !== py) {
        parent.set(px, py);
      }
    };

    // Union pairs
    for (const [id1, id2] of pairs) {
      union(id1, id2);
    }

    // Group by root
    const clusters = new Map<string, string[]>();
    for (const [id1] of pairs) {
      const root = find(id1);
      const cluster = clusters.get(root) ?? [];
      if (!cluster.includes(id1)) cluster.push(id1);
      clusters.set(root, cluster);
    }
    for (const [, id2] of pairs) {
      const root = find(id2);
      const cluster = clusters.get(root) ?? [];
      if (!cluster.includes(id2)) cluster.push(id2);
      clusters.set(root, cluster);
    }

    return [...clusters.values()];
  }

  /**
   * Detect structural patterns (hubs, bridges)
   */
  private detectStructuralPatterns(
    graph: InternalGraph,
    concepts: ExtractedConcept[]
  ): DetectedPattern[] {
    const patterns: DetectedPattern[] = [];

    // Find hub nodes (high degree)
    const hubThreshold = Math.max(3, Math.floor(concepts.length * 0.1));
    const hubs: string[] = [];

    for (const [conceptId, neighbors] of graph.adjacencyList) {
      const outDegree = neighbors.size;
      const inDegree = graph.reverseAdjacencyList.get(conceptId)?.size ?? 0;
      const totalDegree = outDegree + inDegree;

      if (totalDegree >= hubThreshold) {
        hubs.push(conceptId);
      }
    }

    if (hubs.length > 0) {
      patterns.push({
        pattern_id: crypto.randomUUID(),
        pattern_type: 'structural',
        description: `Hub concepts with high connectivity: ${hubs.length} hubs detected`,
        involved_concepts: hubs,
        occurrences: hubs.length,
        confidence: Math.min(0.6 + hubs.length * 0.05, 0.95),
      });
    }

    return patterns;
  }

  /**
   * Calculate graph statistics
   */
  private calculateStatistics(
    concepts: ExtractedConcept[],
    relationships: ExtractedRelationship[],
    patterns: DetectedPattern[],
    graph: InternalGraph
  ): GraphStatistics {
    // Concept type distribution
    const conceptTypeDistribution: Record<string, number> = {};
    for (const concept of concepts) {
      conceptTypeDistribution[concept.type] = (conceptTypeDistribution[concept.type] ?? 0) + 1;
    }

    // Relationship type distribution
    const relationshipTypeDistribution: Record<string, number> = {};
    for (const rel of relationships) {
      relationshipTypeDistribution[rel.relationship_type] =
        (relationshipTypeDistribution[rel.relationship_type] ?? 0) + 1;
    }

    // Average confidences
    const avgConceptConfidence = concepts.length > 0
      ? concepts.reduce((sum, c) => sum + c.confidence, 0) / concepts.length
      : 0;
    const avgRelationshipConfidence = relationships.length > 0
      ? relationships.reduce((sum, r) => sum + r.confidence, 0) / relationships.length
      : 0;

    // Graph density
    const n = concepts.length;
    const maxEdges = n * (n - 1);
    const graphDensity = maxEdges > 0 ? relationships.length / maxEdges : 0;

    // Connected components (simplified)
    const connectedComponents = this.countConnectedComponents(graph);

    return {
      total_concepts: concepts.length,
      total_relationships: relationships.length,
      total_patterns: patterns.length,
      concept_type_distribution: conceptTypeDistribution,
      relationship_type_distribution: relationshipTypeDistribution,
      avg_concept_confidence: avgConceptConfidence,
      avg_relationship_confidence: avgRelationshipConfidence,
      graph_density: graphDensity,
      connected_components: connectedComponents,
    };
  }

  /**
   * Count connected components in the graph
   */
  private countConnectedComponents(graph: InternalGraph): number {
    const visited = new Set<string>();
    let components = 0;

    const dfs = (nodeId: string): void => {
      if (visited.has(nodeId)) return;
      visited.add(nodeId);

      // Visit neighbors
      for (const neighbor of graph.adjacencyList.get(nodeId) ?? []) {
        dfs(neighbor);
      }
      for (const neighbor of graph.reverseAdjacencyList.get(nodeId) ?? []) {
        dfs(neighbor);
      }
    };

    for (const nodeId of graph.concepts.keys()) {
      if (!visited.has(nodeId)) {
        dfs(nodeId);
        components++;
      }
    }

    return components;
  }
}

/**
 * Create a knowledge graph builder instance
 */
export function createGraphBuilder(
  extractionOptions?: Parameters<typeof createExtractor>[0],
  graphOptions?: Partial<GraphOptions>,
  minConfidence?: number
): KnowledgeGraphBuilder {
  return new KnowledgeGraphBuilder(extractionOptions, graphOptions, minConfidence);
}
