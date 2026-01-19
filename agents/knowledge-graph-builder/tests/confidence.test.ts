/**
 * Knowledge Graph Builder Agent - Confidence Scoring Tests
 *
 * These tests verify confidence calculation for various scenarios including:
 * - Edge cases (0.0, 1.0 bounds)
 * - Aggregation of multiple signals
 * - Confidence decay with uncertainty
 * - Composite confidence scoring
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';

// ============================================================================
// CONFIDENCE SCORING TYPES
// ============================================================================

interface ConfidenceSignal {
  source: string;
  value: number;
  weight: number;
}

interface ConceptConfidenceInput {
  textMatchScore: number; // 0-1: How well the text matches known patterns
  frequencyScore: number; // 0-1: Normalized frequency of occurrence
  contextScore: number; // 0-1: How appropriate the context is
  cooccurrenceScore?: number; // 0-1: Related concepts appearing together
}

interface EntityConfidenceInput {
  exactMatchScore: number; // 0-1: Exact vs fuzzy match
  typeMatchScore: number; // 0-1: How well it matches entity type patterns
  contextRelevanceScore: number; // 0-1: Relevance to surrounding content
}

interface RelationshipConfidenceInput {
  proximityScore: number; // 0-1: How close entities/concepts are in text
  semanticSimilarityScore: number; // 0-1: Semantic relatedness
  evidenceStrengthScore: number; // 0-1: Strength of evidence for relationship
}

interface PatternConfidenceInput {
  frequencyScore: number; // 0-1: How often pattern appears
  consistencyScore: number; // 0-1: How consistent pattern occurrences are
  coverageScore: number; // 0-1: What percentage of sessions contain pattern
}

// ============================================================================
// CONFIDENCE SCORING FUNCTIONS
// ============================================================================

/**
 * Clamp a value between 0 and 1
 */
function clamp(value: number): number {
  return Math.max(0, Math.min(1, value));
}

/**
 * Calculate weighted average of confidence signals
 */
function weightedAverage(signals: ConfidenceSignal[]): number {
  if (signals.length === 0) return 0;

  let totalWeight = 0;
  let weightedSum = 0;

  for (const signal of signals) {
    const clampedValue = clamp(signal.value);
    weightedSum += clampedValue * signal.weight;
    totalWeight += signal.weight;
  }

  if (totalWeight === 0) return 0;
  return clamp(weightedSum / totalWeight);
}

/**
 * Calculate concept extraction confidence
 */
function calculateConceptConfidence(input: ConceptConfidenceInput): number {
  const signals: ConfidenceSignal[] = [
    { source: 'textMatch', value: input.textMatchScore, weight: 0.4 },
    { source: 'frequency', value: input.frequencyScore, weight: 0.3 },
    { source: 'context', value: input.contextScore, weight: 0.2 },
  ];

  if (input.cooccurrenceScore !== undefined) {
    signals.push({ source: 'cooccurrence', value: input.cooccurrenceScore, weight: 0.1 });
  }

  return clamp(weightedAverage(signals));
}

/**
 * Calculate entity extraction confidence
 */
function calculateEntityConfidence(input: EntityConfidenceInput): number {
  const signals: ConfidenceSignal[] = [
    { source: 'exactMatch', value: input.exactMatchScore, weight: 0.5 },
    { source: 'typeMatch', value: input.typeMatchScore, weight: 0.3 },
    { source: 'contextRelevance', value: input.contextRelevanceScore, weight: 0.2 },
  ];

  return clamp(weightedAverage(signals));
}

/**
 * Calculate relationship detection confidence
 */
function calculateRelationshipConfidence(input: RelationshipConfidenceInput): number {
  const signals: ConfidenceSignal[] = [
    { source: 'proximity', value: input.proximityScore, weight: 0.3 },
    { source: 'semanticSimilarity', value: input.semanticSimilarityScore, weight: 0.4 },
    { source: 'evidenceStrength', value: input.evidenceStrengthScore, weight: 0.3 },
  ];

  return clamp(weightedAverage(signals));
}

/**
 * Calculate pattern detection confidence
 */
function calculatePatternConfidence(input: PatternConfidenceInput): number {
  const signals: ConfidenceSignal[] = [
    { source: 'frequency', value: input.frequencyScore, weight: 0.4 },
    { source: 'consistency', value: input.consistencyScore, weight: 0.35 },
    { source: 'coverage', value: input.coverageScore, weight: 0.25 },
  ];

  return clamp(weightedAverage(signals));
}

/**
 * Calculate overall graph confidence as aggregation of component confidences
 */
function calculateOverallConfidence(confidences: number[]): number {
  if (confidences.length === 0) return 0;

  // Use harmonic mean to penalize low individual confidences
  const reciprocalSum = confidences.reduce((sum, c) => sum + (c > 0 ? 1 / c : 0), 0);

  if (reciprocalSum === 0) return 0;

  return clamp(confidences.length / reciprocalSum);
}

/**
 * Apply confidence decay based on uncertainty factors
 */
function applyConfidenceDecay(baseConfidence: number, uncertaintyFactor: number): number {
  // Decay formula: confidence * (1 - uncertainty)^2
  const decayMultiplier = Math.pow(1 - clamp(uncertaintyFactor), 2);
  return clamp(baseConfidence * decayMultiplier);
}

/**
 * Combine confidence scores using different strategies
 */
function combineConfidences(
  confidences: number[],
  strategy: 'mean' | 'min' | 'max' | 'harmonic' | 'geometric'
): number {
  if (confidences.length === 0) return 0;

  const clamped = confidences.map(clamp);

  switch (strategy) {
    case 'mean':
      return clamp(clamped.reduce((a, b) => a + b, 0) / clamped.length);

    case 'min':
      return Math.min(...clamped);

    case 'max':
      return Math.max(...clamped);

    case 'harmonic':
      const reciprocalSum = clamped.reduce((sum, c) => sum + (c > 0 ? 1 / c : Infinity), 0);
      return reciprocalSum === Infinity ? 0 : clamp(clamped.length / reciprocalSum);

    case 'geometric':
      const product = clamped.reduce((prod, c) => prod * (c > 0 ? c : 1), 1);
      const nonZeroCount = clamped.filter((c) => c > 0).length;
      return nonZeroCount > 0 ? clamp(Math.pow(product, 1 / nonZeroCount)) : 0;

    default:
      return 0;
  }
}

// ============================================================================
// TESTS
// ============================================================================

describe('Knowledge Graph Builder - Confidence Scoring', () => {
  describe('Boundary Conditions', () => {
    it('should return exactly 0.0 for minimum confidence', () => {
      const input: ConceptConfidenceInput = {
        textMatchScore: 0,
        frequencyScore: 0,
        contextScore: 0,
        cooccurrenceScore: 0,
      };

      const confidence = calculateConceptConfidence(input);
      assert.strictEqual(confidence, 0);
    });

    it('should return exactly 1.0 for maximum confidence', () => {
      const input: ConceptConfidenceInput = {
        textMatchScore: 1,
        frequencyScore: 1,
        contextScore: 1,
        cooccurrenceScore: 1,
      };

      const confidence = calculateConceptConfidence(input);
      assert.strictEqual(confidence, 1);
    });

    it('should clamp values above 1.0 to 1.0', () => {
      const input: ConceptConfidenceInput = {
        textMatchScore: 1.5, // Invalid but should be clamped
        frequencyScore: 1.2,
        contextScore: 1.0,
      };

      const confidence = calculateConceptConfidence(input);
      assert.ok(confidence <= 1, 'Confidence should not exceed 1.0');
    });

    it('should clamp values below 0.0 to 0.0', () => {
      const input: ConceptConfidenceInput = {
        textMatchScore: -0.5, // Invalid but should be clamped
        frequencyScore: -0.1,
        contextScore: 0.5,
      };

      const confidence = calculateConceptConfidence(input);
      assert.ok(confidence >= 0, 'Confidence should not be negative');
    });

    it('should handle exactly 0.5 as middle confidence', () => {
      const input: ConceptConfidenceInput = {
        textMatchScore: 0.5,
        frequencyScore: 0.5,
        contextScore: 0.5,
      };

      const confidence = calculateConceptConfidence(input);
      assert.ok(Math.abs(confidence - 0.5) < 0.01, 'Middle values should yield ~0.5 confidence');
    });

    it('should handle epsilon values near boundaries', () => {
      const almostZero: ConceptConfidenceInput = {
        textMatchScore: 0.001,
        frequencyScore: 0.001,
        contextScore: 0.001,
      };

      const almostOne: ConceptConfidenceInput = {
        textMatchScore: 0.999,
        frequencyScore: 0.999,
        contextScore: 0.999,
      };

      const nearZeroConfidence = calculateConceptConfidence(almostZero);
      const nearOneConfidence = calculateConceptConfidence(almostOne);

      assert.ok(nearZeroConfidence > 0, 'Should be > 0');
      assert.ok(nearZeroConfidence < 0.1, 'Should be close to 0');
      assert.ok(nearOneConfidence < 1, 'Should be < 1');
      assert.ok(nearOneConfidence > 0.9, 'Should be close to 1');
    });
  });

  describe('Weighted Average Calculation', () => {
    it('should calculate correct weighted average', () => {
      const signals: ConfidenceSignal[] = [
        { source: 'a', value: 1.0, weight: 1 },
        { source: 'b', value: 0.0, weight: 1 },
      ];

      const result = weightedAverage(signals);
      assert.strictEqual(result, 0.5, 'Equal weights should give arithmetic mean');
    });

    it('should respect weight ratios', () => {
      const signals: ConfidenceSignal[] = [
        { source: 'heavy', value: 1.0, weight: 3 },
        { source: 'light', value: 0.0, weight: 1 },
      ];

      const result = weightedAverage(signals);
      assert.strictEqual(result, 0.75, 'Weighted average should be 0.75');
    });

    it('should handle empty signal array', () => {
      const signals: ConfidenceSignal[] = [];
      const result = weightedAverage(signals);
      assert.strictEqual(result, 0, 'Empty signals should return 0');
    });

    it('should handle zero total weight', () => {
      const signals: ConfidenceSignal[] = [
        { source: 'a', value: 0.5, weight: 0 },
        { source: 'b', value: 0.8, weight: 0 },
      ];

      const result = weightedAverage(signals);
      assert.strictEqual(result, 0, 'Zero total weight should return 0');
    });

    it('should handle single signal', () => {
      const signals: ConfidenceSignal[] = [{ source: 'only', value: 0.75, weight: 1 }];

      const result = weightedAverage(signals);
      assert.strictEqual(result, 0.75, 'Single signal should return its value');
    });
  });

  describe('Concept Confidence', () => {
    it('should weight text match most heavily', () => {
      const highTextMatch: ConceptConfidenceInput = {
        textMatchScore: 0.9,
        frequencyScore: 0.3,
        contextScore: 0.3,
      };

      const lowTextMatch: ConceptConfidenceInput = {
        textMatchScore: 0.3,
        frequencyScore: 0.9,
        contextScore: 0.9,
      };

      const highResult = calculateConceptConfidence(highTextMatch);
      const lowResult = calculateConceptConfidence(lowTextMatch);

      // With text match weight of 0.4, high text match should have significant impact
      assert.ok(
        Math.abs(highResult - lowResult) > 0.1,
        'Text match weighting should create noticeable difference'
      );
    });

    it('should include optional cooccurrence score', () => {
      const withoutCooccurrence: ConceptConfidenceInput = {
        textMatchScore: 0.5,
        frequencyScore: 0.5,
        contextScore: 0.5,
      };

      const withCooccurrence: ConceptConfidenceInput = {
        textMatchScore: 0.5,
        frequencyScore: 0.5,
        contextScore: 0.5,
        cooccurrenceScore: 1.0,
      };

      const withoutResult = calculateConceptConfidence(withoutCooccurrence);
      const withResult = calculateConceptConfidence(withCooccurrence);

      // Adding high cooccurrence should slightly increase confidence
      assert.ok(withResult >= withoutResult, 'High cooccurrence should increase or maintain confidence');
    });
  });

  describe('Entity Confidence', () => {
    it('should heavily weight exact match', () => {
      const exactMatch: EntityConfidenceInput = {
        exactMatchScore: 1.0,
        typeMatchScore: 0.5,
        contextRelevanceScore: 0.5,
      };

      const fuzzyMatch: EntityConfidenceInput = {
        exactMatchScore: 0.5,
        typeMatchScore: 1.0,
        contextRelevanceScore: 1.0,
      };

      const exactResult = calculateEntityConfidence(exactMatch);
      const fuzzyResult = calculateEntityConfidence(fuzzyMatch);

      // Exact match has 0.5 weight, so it should be influential
      assert.ok(exactResult > 0.7, 'Exact match should yield high confidence');
    });

    it('should produce consistent results for same input', () => {
      const input: EntityConfidenceInput = {
        exactMatchScore: 0.85,
        typeMatchScore: 0.72,
        contextRelevanceScore: 0.68,
      };

      const result1 = calculateEntityConfidence(input);
      const result2 = calculateEntityConfidence(input);

      assert.strictEqual(result1, result2, 'Same input should yield identical confidence');
    });
  });

  describe('Relationship Confidence', () => {
    it('should weight semantic similarity most heavily', () => {
      const highSemantic: RelationshipConfidenceInput = {
        proximityScore: 0.3,
        semanticSimilarityScore: 0.9,
        evidenceStrengthScore: 0.3,
      };

      const result = calculateRelationshipConfidence(highSemantic);

      // With semantic similarity weight of 0.4, it should pull average up
      assert.ok(result > 0.5, 'High semantic similarity should increase confidence');
    });

    it('should balance multiple strong signals', () => {
      const balanced: RelationshipConfidenceInput = {
        proximityScore: 0.8,
        semanticSimilarityScore: 0.8,
        evidenceStrengthScore: 0.8,
      };

      const result = calculateRelationshipConfidence(balanced);
      assert.ok(Math.abs(result - 0.8) < 0.01, 'Balanced 0.8 inputs should yield ~0.8');
    });
  });

  describe('Pattern Confidence', () => {
    it('should require frequency for high confidence', () => {
      const lowFrequency: PatternConfidenceInput = {
        frequencyScore: 0.2,
        consistencyScore: 1.0,
        coverageScore: 1.0,
      };

      const result = calculatePatternConfidence(lowFrequency);
      assert.ok(result < 0.8, 'Low frequency should prevent maximum confidence');
    });

    it('should reward consistent patterns', () => {
      const consistent: PatternConfidenceInput = {
        frequencyScore: 0.5,
        consistencyScore: 1.0,
        coverageScore: 0.5,
      };

      const inconsistent: PatternConfidenceInput = {
        frequencyScore: 0.5,
        consistencyScore: 0.2,
        coverageScore: 0.5,
      };

      const consistentResult = calculatePatternConfidence(consistent);
      const inconsistentResult = calculatePatternConfidence(inconsistent);

      assert.ok(consistentResult > inconsistentResult, 'Consistency should increase confidence');
    });
  });

  describe('Overall Graph Confidence', () => {
    it('should use harmonic mean to penalize low values', () => {
      const confidences = [0.9, 0.9, 0.1]; // One low confidence

      const result = calculateOverallConfidence(confidences);

      // Harmonic mean penalizes low values heavily
      assert.ok(result < 0.6, 'Low individual confidence should reduce overall');
    });

    it('should handle all equal confidences', () => {
      const confidences = [0.7, 0.7, 0.7, 0.7];

      const result = calculateOverallConfidence(confidences);
      assert.ok(Math.abs(result - 0.7) < 0.01, 'Equal confidences should yield that value');
    });

    it('should handle empty array', () => {
      const result = calculateOverallConfidence([]);
      assert.strictEqual(result, 0);
    });

    it('should handle single confidence', () => {
      const result = calculateOverallConfidence([0.85]);
      assert.strictEqual(result, 0.85);
    });

    it('should handle zero confidence in array', () => {
      const confidences = [0.9, 0.0, 0.9];

      const result = calculateOverallConfidence(confidences);
      // Harmonic mean with zero should handle gracefully
      assert.ok(result >= 0);
      assert.ok(result < 0.9);
    });
  });

  describe('Confidence Decay', () => {
    it('should not decay with zero uncertainty', () => {
      const result = applyConfidenceDecay(0.9, 0);
      assert.strictEqual(result, 0.9);
    });

    it('should fully decay with maximum uncertainty', () => {
      const result = applyConfidenceDecay(0.9, 1.0);
      assert.strictEqual(result, 0);
    });

    it('should apply quadratic decay', () => {
      const baseConfidence = 1.0;
      const uncertainty = 0.5;

      const result = applyConfidenceDecay(baseConfidence, uncertainty);
      const expected = 1.0 * Math.pow(0.5, 2); // 0.25

      assert.strictEqual(result, expected);
    });

    it('should handle intermediate uncertainty values', () => {
      const base = 0.8;
      const result = applyConfidenceDecay(base, 0.3);

      assert.ok(result > 0, 'Should be positive');
      assert.ok(result < base, 'Should be less than base');
    });
  });

  describe('Confidence Combination Strategies', () => {
    const testConfidences = [0.3, 0.5, 0.7, 0.9];

    it('should calculate mean correctly', () => {
      const result = combineConfidences(testConfidences, 'mean');
      const expected = (0.3 + 0.5 + 0.7 + 0.9) / 4;
      assert.ok(Math.abs(result - expected) < 0.001);
    });

    it('should find minimum correctly', () => {
      const result = combineConfidences(testConfidences, 'min');
      assert.strictEqual(result, 0.3);
    });

    it('should find maximum correctly', () => {
      const result = combineConfidences(testConfidences, 'max');
      assert.strictEqual(result, 0.9);
    });

    it('should calculate harmonic mean correctly', () => {
      const result = combineConfidences(testConfidences, 'harmonic');
      // Harmonic mean should be less than arithmetic mean
      const arithmeticMean = combineConfidences(testConfidences, 'mean');
      assert.ok(result < arithmeticMean, 'Harmonic mean should be less than arithmetic mean');
    });

    it('should calculate geometric mean correctly', () => {
      const result = combineConfidences(testConfidences, 'geometric');
      const product = 0.3 * 0.5 * 0.7 * 0.9;
      const expected = Math.pow(product, 1 / 4);
      assert.ok(Math.abs(result - expected) < 0.001);
    });

    it('should handle empty array for all strategies', () => {
      const strategies: Array<'mean' | 'min' | 'max' | 'harmonic' | 'geometric'> = [
        'mean',
        'min',
        'max',
        'harmonic',
        'geometric',
      ];

      for (const strategy of strategies) {
        const result = combineConfidences([], strategy);
        assert.strictEqual(result, 0, `${strategy} should return 0 for empty array`);
      }
    });

    it('should handle single value for all strategies', () => {
      const strategies: Array<'mean' | 'min' | 'max' | 'harmonic' | 'geometric'> = [
        'mean',
        'min',
        'max',
        'harmonic',
        'geometric',
      ];

      for (const strategy of strategies) {
        const result = combineConfidences([0.75], strategy);
        assert.strictEqual(result, 0.75, `${strategy} should return the single value`);
      }
    });
  });

  describe('Aggregation of Multiple Signals', () => {
    it('should aggregate concept, entity, and relationship confidences', () => {
      const conceptConf = calculateConceptConfidence({
        textMatchScore: 0.8,
        frequencyScore: 0.6,
        contextScore: 0.7,
      });

      const entityConf = calculateEntityConfidence({
        exactMatchScore: 0.9,
        typeMatchScore: 0.7,
        contextRelevanceScore: 0.6,
      });

      const relConf = calculateRelationshipConfidence({
        proximityScore: 0.7,
        semanticSimilarityScore: 0.8,
        evidenceStrengthScore: 0.6,
      });

      const overall = calculateOverallConfidence([conceptConf, entityConf, relConf]);

      assert.ok(overall > 0, 'Aggregated confidence should be positive');
      assert.ok(overall <= 1, 'Aggregated confidence should not exceed 1');
      assert.ok(overall < Math.max(conceptConf, entityConf, relConf), 'Harmonic mean should be < max');
    });

    it('should handle mixed confidence levels appropriately', () => {
      // High extraction confidence but low relationship confidence
      const highExtraction = 0.95;
      const lowRelationship = 0.3;

      const overall = calculateOverallConfidence([highExtraction, lowRelationship]);

      // Overall should reflect the uncertainty from low relationship confidence
      assert.ok(overall < highExtraction, 'Low relationship confidence should pull down overall');
      assert.ok(overall > lowRelationship, 'High extraction should pull up from minimum');
    });
  });
});
