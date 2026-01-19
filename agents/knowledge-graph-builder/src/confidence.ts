/**
 * Confidence Scoring Module
 *
 * Calculates confidence and relevance scores for extracted knowledge.
 * This module implements deterministic scoring algorithms.
 *
 * EXPLICIT NON-RESPONSIBILITIES:
 * - This module does NOT persist data
 * - This module does NOT make decisions
 * - This module does NOT trigger actions
 */

import type {
  ExtractedConcept,
  ExtractedRelationship,
  DetectedPattern,
  GraphStatistics,
} from './types.js';

/**
 * Confidence factors and their weights
 */
export interface ConfidenceFactors {
  frequency: number;
  contextVariety: number;
  entityTypePresence: number;
  evidenceStrength: number;
  patternConsistency: number;
  cooccurrenceStrength: number;
}

/**
 * Weights for confidence calculation
 */
export const CONFIDENCE_WEIGHTS: Record<keyof ConfidenceFactors, number> = {
  frequency: 0.25,
  contextVariety: 0.20,
  entityTypePresence: 0.15,
  evidenceStrength: 0.20,
  patternConsistency: 0.10,
  cooccurrenceStrength: 0.10,
};

/**
 * Confidence calculation result
 */
export interface ConfidenceResult {
  score: number;
  factors: ConfidenceFactors;
  breakdown: Record<string, number>;
}

/**
 * Confidence calculator for knowledge graph elements
 */
export class ConfidenceCalculator {
  private readonly minConfidence: number;
  private readonly weights: Record<keyof ConfidenceFactors, number>;

  constructor(minConfidence: number = 0.5, weights?: Partial<Record<keyof ConfidenceFactors, number>>) {
    this.minConfidence = minConfidence;
    this.weights = { ...CONFIDENCE_WEIGHTS, ...weights };
  }

  /**
   * Calculate confidence for a concept
   */
  calculateConceptConfidence(
    concept: ExtractedConcept,
    totalTexts: number,
    allConcepts: ExtractedConcept[]
  ): ConfidenceResult {
    const factors: ConfidenceFactors = {
      frequency: this.calculateFrequencyScore(concept.frequency, totalTexts),
      contextVariety: this.calculateContextVarietyScore(concept.source_content_ids.length, totalTexts),
      entityTypePresence: concept.entity_type ? 1.0 : 0.5,
      evidenceStrength: this.calculateEvidenceStrength(concept.context_snippets?.length ?? 0),
      patternConsistency: this.calculatePatternConsistency(concept, allConcepts),
      cooccurrenceStrength: 0.5, // Default, updated by graph builder
    };

    const score = this.computeWeightedScore(factors);
    const breakdown = this.createBreakdown(factors);

    return { score, factors, breakdown };
  }

  /**
   * Calculate confidence for a relationship
   */
  calculateRelationshipConfidence(
    relationship: ExtractedRelationship,
    sourceConcept: ExtractedConcept,
    targetConcept: ExtractedConcept,
    totalTexts: number
  ): ConfidenceResult {
    const evidenceCount = relationship.evidence?.length ?? 0;
    const uniqueSources = new Set(relationship.evidence?.map(e => e.content_id) ?? []).size;

    const factors: ConfidenceFactors = {
      frequency: this.calculateFrequencyScore(relationship.weight, totalTexts),
      contextVariety: this.calculateContextVarietyScore(uniqueSources, totalTexts),
      entityTypePresence: (sourceConcept.entity_type && targetConcept.entity_type) ? 1.0 : 0.6,
      evidenceStrength: this.calculateEvidenceStrength(evidenceCount),
      patternConsistency: this.calculateRelationshipPatternScore(relationship, sourceConcept, targetConcept),
      cooccurrenceStrength: Math.min(sourceConcept.confidence, targetConcept.confidence),
    };

    const score = this.computeWeightedScore(factors);
    const breakdown = this.createBreakdown(factors);

    return { score, factors, breakdown };
  }

  /**
   * Calculate confidence for a detected pattern
   */
  calculatePatternConfidence(
    pattern: DetectedPattern,
    involvedConcepts: ExtractedConcept[],
    totalTexts: number
  ): ConfidenceResult {
    const avgConceptConfidence = involvedConcepts.length > 0
      ? involvedConcepts.reduce((sum, c) => sum + c.confidence, 0) / involvedConcepts.length
      : 0.5;

    const factors: ConfidenceFactors = {
      frequency: this.calculateFrequencyScore(pattern.occurrences, totalTexts),
      contextVariety: pattern.involved_concepts.length / Math.max(involvedConcepts.length, 1),
      entityTypePresence: involvedConcepts.some(c => c.entity_type) ? 0.8 : 0.5,
      evidenceStrength: Math.min(pattern.occurrences / 3, 1.0),
      patternConsistency: this.calculateTemporalConsistency(pattern),
      cooccurrenceStrength: avgConceptConfidence,
    };

    const score = this.computeWeightedScore(factors);
    const breakdown = this.createBreakdown(factors);

    return { score, factors, breakdown };
  }

  /**
   * Calculate overall graph confidence
   */
  calculateOverallConfidence(
    concepts: ExtractedConcept[],
    relationships: ExtractedRelationship[],
    patterns: DetectedPattern[],
    statistics: GraphStatistics
  ): ConfidenceResult {
    if (concepts.length === 0) {
      return {
        score: 0,
        factors: this.createEmptyFactors(),
        breakdown: {},
      };
    }

    const avgConceptConfidence = concepts.reduce((sum, c) => sum + c.confidence, 0) / concepts.length;
    const avgRelationshipConfidence = relationships.length > 0
      ? relationships.reduce((sum, r) => sum + r.confidence, 0) / relationships.length
      : 0;
    const avgPatternConfidence = patterns.length > 0
      ? patterns.reduce((sum, p) => sum + p.confidence, 0) / patterns.length
      : 0;

    const factors: ConfidenceFactors = {
      frequency: Math.min(concepts.length / 10, 1.0),
      contextVariety: this.calculateGraphVariety(statistics),
      entityTypePresence: concepts.filter(c => c.entity_type).length / concepts.length,
      evidenceStrength: avgRelationshipConfidence,
      patternConsistency: avgPatternConfidence,
      cooccurrenceStrength: avgConceptConfidence,
    };

    const score = this.computeWeightedScore(factors);
    const breakdown = this.createBreakdown(factors);

    return { score, factors, breakdown };
  }

  /**
   * Calculate relevance score for a concept in a specific context
   */
  calculateRelevanceScore(
    concept: ExtractedConcept,
    queryContext: string
  ): number {
    const normalizedContext = queryContext.toLowerCase();
    const normalizedName = concept.normalized_name.toLowerCase();

    // Direct mention score
    const directMention = normalizedContext.includes(normalizedName) ? 0.5 : 0;

    // Partial match score
    const words = normalizedName.split(/\s+/);
    const partialMatches = words.filter(w => normalizedContext.includes(w)).length / words.length;
    const partialScore = partialMatches * 0.3;

    // Base confidence contribution
    const confidenceScore = concept.confidence * 0.2;

    return Math.min(directMention + partialScore + confidenceScore, 1.0);
  }

  /**
   * Rank concepts by relevance to a query
   */
  rankConceptsByRelevance(
    concepts: ExtractedConcept[],
    queryContext: string
  ): Array<{ concept: ExtractedConcept; relevance: number }> {
    return concepts
      .map(concept => ({
        concept,
        relevance: this.calculateRelevanceScore(concept, queryContext),
      }))
      .sort((a, b) => b.relevance - a.relevance);
  }

  /**
   * Filter concepts by minimum confidence
   */
  filterByConfidence<T extends { confidence: number }>(
    items: T[],
    minConfidence?: number
  ): T[] {
    const threshold = minConfidence ?? this.minConfidence;
    return items.filter(item => item.confidence >= threshold);
  }

  /**
   * Calculate association strength between two concepts
   */
  calculateAssociationStrength(
    concept1: ExtractedConcept,
    concept2: ExtractedConcept,
    relationship?: ExtractedRelationship
  ): number {
    // Base co-occurrence score
    const sharedSources = concept1.source_content_ids.filter(
      id => concept2.source_content_ids.includes(id)
    ).length;
    const totalSources = new Set([
      ...concept1.source_content_ids,
      ...concept2.source_content_ids,
    ]).size;
    const cooccurrenceScore = totalSources > 0 ? sharedSources / totalSources : 0;

    // Relationship boost
    const relationshipScore = relationship ? relationship.confidence * 0.4 : 0;

    // Confidence contribution
    const confidenceScore = (concept1.confidence + concept2.confidence) / 2 * 0.2;

    return Math.min(cooccurrenceScore * 0.4 + relationshipScore + confidenceScore, 1.0);
  }

  // Private helper methods

  private calculateFrequencyScore(frequency: number, total: number): number {
    if (total === 0) return 0.5;
    const normalized = frequency / total;
    // Use logarithmic scaling to prevent outliers from dominating
    return Math.min(Math.log10(normalized * 10 + 1) / 2, 1.0);
  }

  private calculateContextVarietyScore(uniqueContexts: number, totalContexts: number): number {
    if (totalContexts === 0) return 0.5;
    return Math.min(uniqueContexts / totalContexts, 1.0);
  }

  private calculateEvidenceStrength(evidenceCount: number): number {
    // Diminishing returns for evidence count
    return Math.min(1 - Math.exp(-evidenceCount / 3), 1.0);
  }

  private calculatePatternConsistency(
    concept: ExtractedConcept,
    allConcepts: ExtractedConcept[]
  ): number {
    // Check how many other concepts share the same type
    const sameType = allConcepts.filter(c => c.type === concept.type).length;
    const ratio = allConcepts.length > 0 ? sameType / allConcepts.length : 0.5;
    // Higher consistency if concept type is neither too common nor too rare
    return 1 - Math.abs(ratio - 0.3);
  }

  private calculateRelationshipPatternScore(
    relationship: ExtractedRelationship,
    source: ExtractedConcept,
    target: ExtractedConcept
  ): number {
    // Score based on relationship type appropriateness
    const typeScores: Record<string, number> = {
      is_a: source.type === 'entity' && target.type === 'entity' ? 0.9 : 0.5,
      has_attribute: source.type === 'entity' && target.type === 'attribute' ? 0.9 : 0.5,
      related_to: 0.6, // Generic relationship
      part_of: 0.7,
      causes: source.type === 'action' || target.type === 'action' ? 0.8 : 0.5,
      precedes: 0.7,
      co_occurs: 0.5,
      depends_on: 0.7,
      references: 0.6,
    };

    return typeScores[relationship.relationship_type] ?? 0.5;
  }

  private calculateTemporalConsistency(pattern: DetectedPattern): number {
    if (!pattern.first_seen || !pattern.last_seen) return 0.5;

    const first = new Date(pattern.first_seen).getTime();
    const last = new Date(pattern.last_seen).getTime();
    const duration = last - first;

    // Patterns observed over longer periods are more consistent
    const dayInMs = 24 * 60 * 60 * 1000;
    return Math.min(duration / (7 * dayInMs), 1.0); // Cap at 1 week
  }

  private calculateGraphVariety(statistics: GraphStatistics): number {
    const typeCount = Object.keys(statistics.concept_type_distribution).length;
    const relationTypeCount = Object.keys(statistics.relationship_type_distribution).length;

    // More variety indicates richer graph
    const conceptVariety = Math.min(typeCount / 6, 1.0); // 6 concept types
    const relationVariety = Math.min(relationTypeCount / 9, 1.0); // 9 relationship types

    return (conceptVariety + relationVariety) / 2;
  }

  private computeWeightedScore(factors: ConfidenceFactors): number {
    let score = 0;
    let totalWeight = 0;

    for (const [key, weight] of Object.entries(this.weights)) {
      const factor = factors[key as keyof ConfidenceFactors];
      score += factor * weight;
      totalWeight += weight;
    }

    return totalWeight > 0 ? score / totalWeight : 0;
  }

  private createBreakdown(factors: ConfidenceFactors): Record<string, number> {
    const breakdown: Record<string, number> = {};

    for (const [key, weight] of Object.entries(this.weights)) {
      const factor = factors[key as keyof ConfidenceFactors];
      breakdown[key] = factor * weight;
    }

    return breakdown;
  }

  private createEmptyFactors(): ConfidenceFactors {
    return {
      frequency: 0,
      contextVariety: 0,
      entityTypePresence: 0,
      evidenceStrength: 0,
      patternConsistency: 0,
      cooccurrenceStrength: 0,
    };
  }
}

/**
 * Create a confidence calculator instance
 */
export function createConfidenceCalculator(
  minConfidence?: number,
  weights?: Partial<Record<keyof ConfidenceFactors, number>>
): ConfidenceCalculator {
  return new ConfidenceCalculator(minConfidence, weights);
}
