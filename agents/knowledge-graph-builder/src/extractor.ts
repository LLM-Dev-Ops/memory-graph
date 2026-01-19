/**
 * Knowledge Extractor Module
 *
 * Extracts concepts, entities, and relationships from text content.
 * This module performs stateless extraction operations.
 *
 * EXPLICIT NON-RESPONSIBILITIES:
 * - This module does NOT persist data
 * - This module does NOT modify system behavior
 * - This module does NOT trigger external actions
 */

import {
  type TextContent,
  type ExtractionOptions,
  type ExtractedConcept,
  type ExtractedRelationship,
  type ConceptType,
  type EntityType,
  type RelationshipType,
  STOP_WORDS,
  ENTITY_PATTERNS,
  RELATIONSHIP_PATTERNS,
} from './types.js';

/**
 * Raw extraction result before processing
 */
export interface RawExtractionResult {
  concepts: Map<string, RawConcept>;
  relationships: Array<RawRelationship>;
  entityMentions: Map<string, EntityMention[]>;
}

/**
 * Raw concept before normalization
 */
interface RawConcept {
  name: string;
  type: ConceptType;
  entityType?: EntityType;
  mentions: Array<{
    contentId: string;
    position: number;
    context: string;
  }>;
}

/**
 * Raw relationship before processing
 */
interface RawRelationship {
  sourceName: string;
  targetName: string;
  type: RelationshipType;
  contentId: string;
  snippet: string;
}

/**
 * Entity mention with context
 */
interface EntityMention {
  value: string;
  type: EntityType;
  contentId: string;
  position: number;
  context: string;
}

/**
 * Knowledge Extractor
 *
 * Performs stateless extraction of concepts, entities, and relationships
 * from text content.
 */
export class KnowledgeExtractor {
  private readonly options: Required<ExtractionOptions>;

  constructor(options: Partial<ExtractionOptions> = {}) {
    this.options = {
      extract_entities: options.extract_entities ?? true,
      extract_concepts: options.extract_concepts ?? true,
      extract_relationships: options.extract_relationships ?? true,
      detect_patterns: options.detect_patterns ?? true,
      min_confidence: options.min_confidence ?? 0.5,
      max_concepts_per_text: options.max_concepts_per_text ?? 50,
      entity_types: options.entity_types ?? [],
    };
  }

  /**
   * Extract knowledge from multiple text contents
   */
  extract(texts: TextContent[]): RawExtractionResult {
    const concepts = new Map<string, RawConcept>();
    const relationships: RawRelationship[] = [];
    const entityMentions = new Map<string, EntityMention[]>();

    for (const text of texts) {
      // Extract entities if enabled
      if (this.options.extract_entities) {
        const entities = this.extractEntities(text);
        for (const entity of entities) {
          const key = this.normalizeText(entity.value);
          const existing = entityMentions.get(key) ?? [];
          existing.push(entity);
          entityMentions.set(key, existing);

          // Add entity as concept
          this.addConcept(concepts, {
            name: entity.value,
            type: 'entity',
            entityType: entity.type,
            mentions: [{
              contentId: text.content_id,
              position: entity.position,
              context: entity.context,
            }],
          });
        }
      }

      // Extract concepts (keywords, topics) if enabled
      if (this.options.extract_concepts) {
        const extractedConcepts = this.extractConcepts(text);
        for (const concept of extractedConcepts) {
          this.addConcept(concepts, concept);
        }
      }

      // Extract relationships if enabled
      if (this.options.extract_relationships) {
        const extractedRelations = this.extractRelationships(text, concepts);
        relationships.push(...extractedRelations);
      }
    }

    return { concepts, relationships, entityMentions };
  }

  /**
   * Convert raw extraction to final ExtractedConcept array
   */
  finalizeConcepts(raw: RawExtractionResult): ExtractedConcept[] {
    const concepts: ExtractedConcept[] = [];

    for (const [normalizedName, rawConcept] of raw.concepts) {
      const frequency = rawConcept.mentions.length;
      const confidence = this.calculateConceptConfidence(rawConcept);

      if (confidence < this.options.min_confidence) {
        continue;
      }

      concepts.push({
        concept_id: crypto.randomUUID(),
        name: rawConcept.name,
        normalized_name: normalizedName,
        type: rawConcept.type,
        entity_type: rawConcept.entityType,
        confidence,
        frequency,
        source_content_ids: [...new Set(rawConcept.mentions.map(m => m.contentId))],
        context_snippets: rawConcept.mentions
          .slice(0, 5)
          .map(m => m.context)
          .filter(Boolean),
      });
    }

    // Sort by frequency and confidence
    concepts.sort((a, b) => {
      const scoreA = a.frequency * a.confidence;
      const scoreB = b.frequency * b.confidence;
      return scoreB - scoreA;
    });

    return concepts.slice(0, this.options.max_concepts_per_text * 2);
  }

  /**
   * Convert raw relationships to final ExtractedRelationship array
   */
  finalizeRelationships(
    raw: RawExtractionResult,
    conceptMap: Map<string, string> // normalized_name -> concept_id
  ): ExtractedRelationship[] {
    const relationships: ExtractedRelationship[] = [];
    const relationshipGroups = new Map<string, RawRelationship[]>();

    // Group similar relationships
    for (const rel of raw.relationships) {
      const sourceNorm = this.normalizeText(rel.sourceName);
      const targetNorm = this.normalizeText(rel.targetName);

      // Skip if concepts not found
      const sourceId = conceptMap.get(sourceNorm);
      const targetId = conceptMap.get(targetNorm);
      if (!sourceId || !targetId) continue;

      const key = `${sourceId}:${rel.type}:${targetId}`;
      const existing = relationshipGroups.get(key) ?? [];
      existing.push(rel);
      relationshipGroups.set(key, existing);
    }

    // Create final relationships
    for (const [key, group] of relationshipGroups) {
      const [sourceId, type, targetId] = key.split(':');
      if (!sourceId || !type || !targetId) continue;

      const confidence = this.calculateRelationshipConfidence(group);
      if (confidence < this.options.min_confidence) continue;

      relationships.push({
        relationship_id: crypto.randomUUID(),
        source_concept_id: sourceId,
        target_concept_id: targetId,
        relationship_type: type as RelationshipType,
        confidence,
        weight: group.length,
        evidence: group.slice(0, 3).map(r => ({
          content_id: r.contentId,
          snippet: r.snippet,
        })),
      });
    }

    return relationships;
  }

  /**
   * Extract named entities from text
   */
  private extractEntities(text: TextContent): EntityMention[] {
    const entities: EntityMention[] = [];
    const content = text.text;
    const allowedTypes = this.options.entity_types;

    for (const [entityType, patterns] of Object.entries(ENTITY_PATTERNS)) {
      // Skip if entity type not in allowed list
      if (allowedTypes && !allowedTypes.includes(entityType as EntityType)) {
        continue;
      }

      for (const pattern of patterns) {
        // Reset regex state
        pattern.lastIndex = 0;

        let match: RegExpExecArray | null;
        while ((match = pattern.exec(content)) !== null) {
          const value = match[0].trim();

          // Skip short matches and stop words
          if (value.length < 2 || STOP_WORDS.has(value.toLowerCase())) {
            continue;
          }

          entities.push({
            value,
            type: entityType as EntityType,
            contentId: text.content_id,
            position: match.index,
            context: this.extractContext(content, match.index, value.length),
          });
        }
      }
    }

    return this.deduplicateEntities(entities);
  }

  /**
   * Extract concepts (keywords, topics) from text
   */
  private extractConcepts(text: TextContent): RawConcept[] {
    const concepts: RawConcept[] = [];
    const content = text.text;

    // Extract significant words and phrases
    const words = this.tokenize(content);
    const wordFrequency = new Map<string, number>();

    for (const word of words) {
      const normalized = this.normalizeText(word);
      if (normalized.length < 3 || STOP_WORDS.has(normalized)) {
        continue;
      }
      wordFrequency.set(normalized, (wordFrequency.get(normalized) ?? 0) + 1);
    }

    // Extract significant n-grams (2-3 word phrases)
    const ngrams = this.extractNgrams(words, 2, 3);
    for (const ngram of ngrams) {
      const normalized = this.normalizeText(ngram);
      if (normalized.length < 5) continue;
      wordFrequency.set(normalized, (wordFrequency.get(normalized) ?? 0) + 1);
    }

    // Convert to concepts
    for (const [normalized, freq] of wordFrequency) {
      if (freq < 1) continue;

      const position = content.toLowerCase().indexOf(normalized);
      const type = this.inferConceptType(normalized, content);

      concepts.push({
        name: normalized,
        type,
        mentions: [{
          contentId: text.content_id,
          position: position >= 0 ? position : 0,
          context: position >= 0
            ? this.extractContext(content, position, normalized.length)
            : '',
        }],
      });
    }

    return concepts;
  }

  /**
   * Extract relationships between concepts
   */
  private extractRelationships(
    text: TextContent,
    existingConcepts: Map<string, RawConcept>
  ): RawRelationship[] {
    const relationships: RawRelationship[] = [];
    const content = text.text;

    // Pattern-based relationship extraction
    for (const { pattern, type, sourceGroup, targetGroup } of RELATIONSHIP_PATTERNS) {
      pattern.lastIndex = 0;

      let match: RegExpExecArray | null;
      while ((match = pattern.exec(content)) !== null) {
        const source = match[sourceGroup]?.trim();
        const target = match[targetGroup]?.trim();

        if (!source || !target) continue;
        if (source.length < 2 || target.length < 2) continue;
        if (STOP_WORDS.has(source.toLowerCase()) || STOP_WORDS.has(target.toLowerCase())) {
          continue;
        }

        relationships.push({
          sourceName: source,
          targetName: target,
          type,
          contentId: text.content_id,
          snippet: this.extractContext(content, match.index, match[0].length),
        });
      }
    }

    // Co-occurrence based relationships
    const conceptNames = [...existingConcepts.keys()];
    const cooccurrences = this.findCooccurrences(content, conceptNames);
    for (const [source, target] of cooccurrences) {
      relationships.push({
        sourceName: source,
        targetName: target,
        type: 'co_occurs',
        contentId: text.content_id,
        snippet: this.extractContext(
          content,
          content.toLowerCase().indexOf(source.toLowerCase()),
          100
        ),
      });
    }

    return relationships;
  }

  /**
   * Find co-occurring concepts within a window
   */
  private findCooccurrences(
    content: string,
    conceptNames: string[],
    windowSize: number = 100
  ): Array<[string, string]> {
    const cooccurrences: Array<[string, string]> = [];
    const lowerContent = content.toLowerCase();

    for (let i = 0; i < conceptNames.length; i++) {
      const concept1 = conceptNames[i];
      if (!concept1) continue;

      const pos1 = lowerContent.indexOf(concept1);
      if (pos1 === -1) continue;

      for (let j = i + 1; j < conceptNames.length; j++) {
        const concept2 = conceptNames[j];
        if (!concept2) continue;

        const pos2 = lowerContent.indexOf(concept2);
        if (pos2 === -1) continue;

        if (Math.abs(pos1 - pos2) <= windowSize) {
          cooccurrences.push([concept1, concept2]);
        }
      }
    }

    return cooccurrences;
  }

  /**
   * Add concept to map, merging with existing
   */
  private addConcept(concepts: Map<string, RawConcept>, concept: RawConcept): void {
    const normalized = this.normalizeText(concept.name);
    const existing = concepts.get(normalized);

    if (existing) {
      existing.mentions.push(...concept.mentions);
      // Keep the more specific type
      if (concept.entityType && !existing.entityType) {
        existing.entityType = concept.entityType;
        existing.type = 'entity';
      }
    } else {
      concepts.set(normalized, { ...concept, name: concept.name });
    }
  }

  /**
   * Normalize text for comparison
   */
  private normalizeText(text: string): string {
    return text
      .toLowerCase()
      .replace(/[^\w\s-]/g, '')
      .replace(/\s+/g, ' ')
      .trim();
  }

  /**
   * Tokenize text into words
   */
  private tokenize(text: string): string[] {
    return text
      .replace(/[^\w\s'-]/g, ' ')
      .split(/\s+/)
      .filter(w => w.length > 0);
  }

  /**
   * Extract n-grams from words
   */
  private extractNgrams(words: string[], minN: number, maxN: number): string[] {
    const ngrams: string[] = [];

    for (let n = minN; n <= maxN; n++) {
      for (let i = 0; i <= words.length - n; i++) {
        const ngram = words.slice(i, i + n).join(' ');
        // Check if any word in ngram is a stop word
        const hasStopWord = words.slice(i, i + n).some(w =>
          STOP_WORDS.has(w.toLowerCase())
        );
        if (!hasStopWord) {
          ngrams.push(ngram);
        }
      }
    }

    return ngrams;
  }

  /**
   * Extract context around a position
   */
  private extractContext(content: string, position: number, length: number): string {
    const contextWindow = 50;
    const start = Math.max(0, position - contextWindow);
    const end = Math.min(content.length, position + length + contextWindow);
    return content.slice(start, end).replace(/\s+/g, ' ').trim();
  }

  /**
   * Infer concept type from text
   */
  private inferConceptType(concept: string, context: string): ConceptType {
    // Check for action words (verbs)
    const actionPatterns = [/ing$/, /ed$/, /tion$/, /ment$/];
    if (actionPatterns.some(p => p.test(concept))) {
      return 'action';
    }

    // Check for attribute patterns
    const attributePatterns = [/ness$/, /ity$/, /ful$/, /less$/, /able$/];
    if (attributePatterns.some(p => p.test(concept))) {
      return 'attribute';
    }

    // Check frequency for topic vs keyword
    const occurrences = (context.match(new RegExp(concept, 'gi')) ?? []).length;
    if (occurrences >= 3) {
      return 'topic';
    }

    return 'keyword';
  }

  /**
   * Calculate confidence score for a concept
   */
  private calculateConceptConfidence(concept: RawConcept): number {
    let confidence = 0.5;

    // Frequency boost
    const frequency = concept.mentions.length;
    confidence += Math.min(frequency * 0.1, 0.3);

    // Entity type boost
    if (concept.entityType) {
      confidence += 0.1;
    }

    // Context variety boost
    const uniqueContexts = new Set(concept.mentions.map(m => m.contentId)).size;
    confidence += Math.min(uniqueContexts * 0.05, 0.1);

    return Math.min(confidence, 1.0);
  }

  /**
   * Calculate confidence score for a relationship
   */
  private calculateRelationshipConfidence(group: RawRelationship[]): number {
    let confidence = 0.4;

    // Evidence count boost
    confidence += Math.min(group.length * 0.15, 0.4);

    // Multiple sources boost
    const uniqueSources = new Set(group.map(r => r.contentId)).size;
    confidence += Math.min(uniqueSources * 0.1, 0.2);

    return Math.min(confidence, 1.0);
  }

  /**
   * Deduplicate entity mentions
   */
  private deduplicateEntities(entities: EntityMention[]): EntityMention[] {
    const seen = new Map<string, EntityMention>();

    for (const entity of entities) {
      const key = `${entity.value}:${entity.contentId}:${entity.position}`;
      if (!seen.has(key)) {
        seen.set(key, entity);
      }
    }

    return [...seen.values()];
  }
}

/**
 * Create a knowledge extractor instance
 */
export function createExtractor(options?: Partial<ExtractionOptions>): KnowledgeExtractor {
  return new KnowledgeExtractor(options);
}
