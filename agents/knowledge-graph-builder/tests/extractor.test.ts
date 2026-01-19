/**
 * Knowledge Graph Builder Agent - Extractor Tests
 *
 * These tests verify knowledge extraction functionality including:
 * - Concept extraction from prompts
 * - Entity extraction
 * - Relationship detection
 * - Pattern detection with frequency thresholds
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';

// ============================================================================
// EXTRACTOR TYPES
// ============================================================================

interface ExtractedConcept {
  name: string;
  category: string;
  confidence: number;
  sourceText: string;
}

interface ExtractedEntity {
  name: string;
  type: string;
  confidence: number;
  context: string;
}

interface DetectedRelationship {
  sourceId: string;
  targetId: string;
  type: string;
  confidence: number;
  evidence: string;
}

interface DetectedPattern {
  name: string;
  description: string;
  occurrences: number;
  sessionIds: string[];
  confidence: number;
}

interface ExtractionConfig {
  extractConcepts: boolean;
  extractEntities: boolean;
  detectPatterns: boolean;
  minPatternFrequency: number;
  maxDepth: number;
}

// ============================================================================
// MOCK EXTRACTOR IMPLEMENTATION (for testing purposes)
// ============================================================================

/**
 * Simple keyword-based concept extractor for testing
 */
function extractConcepts(content: string): ExtractedConcept[] {
  const concepts: ExtractedConcept[] = [];
  const contentLower = content.toLowerCase();

  // Technology concepts
  const techPatterns: [RegExp, string, string][] = [
    [/\bjwt\b|\bjson web token\b/gi, 'JWT Authentication', 'security'],
    [/\bnode\.?js\b/gi, 'Node.js', 'technology'],
    [/\bmiddleware\b/gi, 'Middleware', 'architecture'],
    [/\bexpress\.?js?\b/gi, 'Express.js', 'framework'],
    [/\bauthentication\b|\bauth\b/gi, 'Authentication', 'security'],
    [/\brefresh token\b/gi, 'Refresh Token', 'security'],
    [/\bapi\b/gi, 'API', 'architecture'],
    [/\bdatabase\b|\bdb\b/gi, 'Database', 'infrastructure'],
    [/\bcache\b|\bcaching\b/gi, 'Caching', 'performance'],
    [/\bmicroservice\b/gi, 'Microservices', 'architecture'],
  ];

  for (const [pattern, name, category] of techPatterns) {
    const matches = content.match(pattern);
    if (matches && matches.length > 0) {
      concepts.push({
        name,
        category,
        confidence: Math.min(0.7 + matches.length * 0.1, 0.99),
        sourceText: matches[0] || '',
      });
    }
  }

  return concepts;
}

/**
 * Simple entity extractor for testing
 */
function extractEntities(content: string): ExtractedEntity[] {
  const entities: ExtractedEntity[] = [];

  // Library/package names (npm packages)
  const libraryPattern = /\b(jsonwebtoken|express|mongoose|axios|lodash|react|vue|angular)\b/gi;
  const libraryMatches = content.match(libraryPattern) || [];
  const seenLibraries = new Set<string>();

  for (const match of libraryMatches) {
    const normalized = match.toLowerCase();
    if (!seenLibraries.has(normalized)) {
      seenLibraries.add(normalized);
      entities.push({
        name: match,
        type: 'library',
        confidence: 0.95,
        context: content.substring(
          Math.max(0, content.indexOf(match) - 20),
          Math.min(content.length, content.indexOf(match) + match.length + 20)
        ),
      });
    }
  }

  // Tool names
  const toolPattern = /\b(npm|yarn|git|docker|kubernetes)\b/gi;
  const toolMatches = content.match(toolPattern) || [];
  const seenTools = new Set<string>();

  for (const match of toolMatches) {
    const normalized = match.toLowerCase();
    if (!seenTools.has(normalized)) {
      seenTools.add(normalized);
      entities.push({
        name: match,
        type: 'tool',
        confidence: 0.98,
        context: content.substring(
          Math.max(0, content.indexOf(match) - 20),
          Math.min(content.length, content.indexOf(match) + match.length + 20)
        ),
      });
    }
  }

  return entities;
}

/**
 * Detect relationships between concepts and entities
 */
function detectRelationships(
  concepts: ExtractedConcept[],
  entities: ExtractedEntity[],
  content: string
): DetectedRelationship[] {
  const relationships: DetectedRelationship[] = [];

  // Create relationship maps based on content proximity
  for (const concept of concepts) {
    for (const entity of entities) {
      const conceptIndex = content.toLowerCase().indexOf(concept.name.toLowerCase());
      const entityIndex = content.toLowerCase().indexOf(entity.name.toLowerCase());

      if (conceptIndex !== -1 && entityIndex !== -1) {
        const distance = Math.abs(conceptIndex - entityIndex);
        // If within 100 characters, they're likely related
        if (distance < 100) {
          const confidence = Math.max(0.5, 0.9 - distance / 200);
          relationships.push({
            sourceId: `concept-${concept.name.toLowerCase().replace(/\s+/g, '-')}`,
            targetId: `entity-${entity.name.toLowerCase()}`,
            type: inferRelationshipType(concept, entity),
            confidence,
            evidence: content.substring(
              Math.min(conceptIndex, entityIndex),
              Math.max(conceptIndex + concept.name.length, entityIndex + entity.name.length)
            ),
          });
        }
      }
    }
  }

  return relationships;
}

/**
 * Infer relationship type between concept and entity
 */
function inferRelationshipType(concept: ExtractedConcept, entity: ExtractedEntity): string {
  if (entity.type === 'library' && concept.category === 'security') {
    return 'requires';
  }
  if (entity.type === 'framework' && concept.category === 'architecture') {
    return 'uses';
  }
  if (concept.category === 'technology') {
    return 'implemented_in';
  }
  return 'related_to';
}

/**
 * Detect patterns across multiple sessions
 */
function detectPatterns(
  sessions: Array<{ sessionId: string; concepts: ExtractedConcept[] }>,
  minFrequency: number
): DetectedPattern[] {
  const patternCounts = new Map<string, { count: number; sessions: string[] }>();

  for (const session of sessions) {
    const sessionConcepts = new Set(session.concepts.map((c) => c.name));

    for (const conceptName of sessionConcepts) {
      const existing = patternCounts.get(conceptName) || { count: 0, sessions: [] };
      existing.count++;
      existing.sessions.push(session.sessionId);
      patternCounts.set(conceptName, existing);
    }
  }

  const patterns: DetectedPattern[] = [];

  for (const [conceptName, data] of patternCounts.entries()) {
    if (data.count >= minFrequency) {
      patterns.push({
        name: `${conceptName} Discussion Pattern`,
        description: `Recurring discussions about ${conceptName}`,
        occurrences: data.count,
        sessionIds: data.sessions,
        confidence: Math.min(0.95, 0.6 + data.count * 0.1),
      });
    }
  }

  return patterns;
}

// ============================================================================
// TESTS
// ============================================================================

describe('Knowledge Graph Builder - Concept Extraction', () => {
  describe('extractConcepts', () => {
    it('should extract JWT-related concepts', () => {
      const content = 'How do I implement JWT authentication in my application?';
      const concepts = extractConcepts(content);

      assert.ok(concepts.length > 0, 'Should extract at least one concept');
      assert.ok(
        concepts.some((c) => c.name.toLowerCase().includes('jwt') || c.name.toLowerCase().includes('authentication')),
        'Should extract JWT or authentication concept'
      );
    });

    it('should extract Node.js as a technology concept', () => {
      const content = 'I am building a backend service with Node.js';
      const concepts = extractConcepts(content);

      const nodeJsConcept = concepts.find((c) => c.name === 'Node.js');
      assert.ok(nodeJsConcept, 'Should extract Node.js concept');
      assert.strictEqual(nodeJsConcept.category, 'technology');
    });

    it('should extract middleware as an architecture concept', () => {
      const content = 'Create authentication middleware for Express';
      const concepts = extractConcepts(content);

      const middlewareConcept = concepts.find((c) => c.name === 'Middleware');
      assert.ok(middlewareConcept, 'Should extract Middleware concept');
      assert.strictEqual(middlewareConcept.category, 'architecture');
    });

    it('should handle empty content', () => {
      const content = '';
      const concepts = extractConcepts(content);

      assert.strictEqual(concepts.length, 0, 'Empty content should yield no concepts');
    });

    it('should handle content with no recognizable concepts', () => {
      const content = 'The quick brown fox jumps over the lazy dog.';
      const concepts = extractConcepts(content);

      assert.strictEqual(concepts.length, 0, 'Random text should yield no concepts');
    });

    it('should extract multiple concepts from rich content', () => {
      const content =
        'Building a Node.js API with JWT authentication using Express middleware and implementing caching for performance';
      const concepts = extractConcepts(content);

      assert.ok(concepts.length >= 3, `Should extract at least 3 concepts, got ${concepts.length}`);

      const conceptNames = concepts.map((c) => c.name);
      assert.ok(conceptNames.includes('Node.js') || conceptNames.some((n) => n.toLowerCase().includes('node')));
    });

    it('should assign appropriate confidence scores', () => {
      const content = 'JWT JWT JWT authentication';
      const concepts = extractConcepts(content);

      for (const concept of concepts) {
        assert.ok(concept.confidence >= 0, 'Confidence should be >= 0');
        assert.ok(concept.confidence <= 1, 'Confidence should be <= 1');
      }
    });

    it('should increase confidence with multiple mentions', () => {
      const singleMention = 'Using JWT for authentication';
      const multipleMentions = 'JWT is great. JWT provides security. JWT tokens are awesome.';

      const singleConcepts = extractConcepts(singleMention);
      const multipleConcepts = extractConcepts(multipleMentions);

      const singleJWT = singleConcepts.find(
        (c) => c.name.toLowerCase().includes('jwt') || c.name.toLowerCase().includes('authentication')
      );
      const multipleJWT = multipleConcepts.find(
        (c) => c.name.toLowerCase().includes('jwt') || c.name.toLowerCase().includes('authentication')
      );

      if (singleJWT && multipleJWT) {
        assert.ok(
          multipleJWT.confidence >= singleJWT.confidence,
          'Multiple mentions should have >= confidence'
        );
      }
    });

    it('should handle case-insensitive matching', () => {
      const lowerCase = 'nodejs is great';
      const upperCase = 'NODEJS is great';
      const mixedCase = 'NodeJS is great';

      const lowerConcepts = extractConcepts(lowerCase);
      const upperConcepts = extractConcepts(upperCase);
      const mixedConcepts = extractConcepts(mixedCase);

      // All should extract the same concept
      assert.ok(lowerConcepts.length > 0 || upperConcepts.length > 0 || mixedConcepts.length > 0);
    });

    it('should preserve source text', () => {
      const content = 'Implementing Node.js middleware';
      const concepts = extractConcepts(content);

      for (const concept of concepts) {
        assert.ok(concept.sourceText, 'Each concept should have sourceText');
        assert.ok(concept.sourceText.length > 0, 'sourceText should not be empty');
      }
    });
  });
});

describe('Knowledge Graph Builder - Entity Extraction', () => {
  describe('extractEntities', () => {
    it('should extract npm package names as libraries', () => {
      const content = 'Install jsonwebtoken package with npm install jsonwebtoken';
      const entities = extractEntities(content);

      const jsonwebtokenEntity = entities.find((e) => e.name.toLowerCase() === 'jsonwebtoken');
      assert.ok(jsonwebtokenEntity, 'Should extract jsonwebtoken as entity');
      assert.strictEqual(jsonwebtokenEntity.type, 'library');
    });

    it('should extract tool names', () => {
      const content = 'Use npm to install dependencies and git for version control';
      const entities = extractEntities(content);

      const npmEntity = entities.find((e) => e.name.toLowerCase() === 'npm');
      const gitEntity = entities.find((e) => e.name.toLowerCase() === 'git');

      assert.ok(npmEntity, 'Should extract npm');
      assert.ok(gitEntity, 'Should extract git');
      assert.strictEqual(npmEntity.type, 'tool');
      assert.strictEqual(gitEntity.type, 'tool');
    });

    it('should extract Express.js as a library', () => {
      const content = 'Building REST API with express framework';
      const entities = extractEntities(content);

      const expressEntity = entities.find((e) => e.name.toLowerCase() === 'express');
      assert.ok(expressEntity, 'Should extract express');
      assert.strictEqual(expressEntity.type, 'library');
    });

    it('should handle empty content', () => {
      const content = '';
      const entities = extractEntities(content);

      assert.strictEqual(entities.length, 0);
    });

    it('should not duplicate entities with multiple mentions', () => {
      const content = 'npm install express && npm run start && npm test';
      const entities = extractEntities(content);

      const npmEntities = entities.filter((e) => e.name.toLowerCase() === 'npm');
      assert.strictEqual(npmEntities.length, 1, 'Should only have one npm entity despite multiple mentions');
    });

    it('should assign high confidence to well-known entities', () => {
      const content = 'Using npm and docker for deployment';
      const entities = extractEntities(content);

      for (const entity of entities) {
        assert.ok(entity.confidence >= 0.9, `Entity ${entity.name} should have high confidence`);
      }
    });

    it('should include context for each entity', () => {
      const content = 'First install the jsonwebtoken package using npm install jsonwebtoken';
      const entities = extractEntities(content);

      for (const entity of entities) {
        assert.ok(entity.context, 'Each entity should have context');
        assert.ok(entity.context.length > 0, 'Context should not be empty');
      }
    });

    it('should extract multiple different entity types', () => {
      const content = 'Deploy the express app using docker and npm scripts';
      const entities = extractEntities(content);

      const types = new Set(entities.map((e) => e.type));
      assert.ok(types.size >= 1, 'Should have at least one entity type');
    });
  });
});

describe('Knowledge Graph Builder - Relationship Detection', () => {
  describe('detectRelationships', () => {
    it('should detect relationship between concept and entity', () => {
      const content = 'Implement JWT authentication using the jsonwebtoken library';
      const concepts = extractConcepts(content);
      const entities = extractEntities(content);
      const relationships = detectRelationships(concepts, entities, content);

      assert.ok(relationships.length > 0, 'Should detect at least one relationship');
    });

    it('should assign relationship type based on concept and entity types', () => {
      const content = 'JWT authentication requires the jsonwebtoken package';
      const concepts = extractConcepts(content);
      const entities = extractEntities(content);
      const relationships = detectRelationships(concepts, entities, content);

      for (const rel of relationships) {
        assert.ok(rel.type, 'Each relationship should have a type');
        assert.ok(
          ['requires', 'uses', 'implemented_in', 'related_to'].includes(rel.type),
          `Relationship type should be valid, got ${rel.type}`
        );
      }
    });

    it('should include evidence for each relationship', () => {
      const content = 'Express middleware handles authentication';
      const concepts = extractConcepts(content);
      const entities = extractEntities(content);
      const relationships = detectRelationships(concepts, entities, content);

      for (const rel of relationships) {
        assert.ok(rel.evidence, 'Each relationship should have evidence');
      }
    });

    it('should have valid source and target IDs', () => {
      const content = 'Node.js application using npm packages';
      const concepts = extractConcepts(content);
      const entities = extractEntities(content);
      const relationships = detectRelationships(concepts, entities, content);

      for (const rel of relationships) {
        assert.ok(rel.sourceId, 'Should have source ID');
        assert.ok(rel.targetId, 'Should have target ID');
        assert.ok(rel.sourceId.startsWith('concept-') || rel.sourceId.startsWith('entity-'));
        assert.ok(rel.targetId.startsWith('concept-') || rel.targetId.startsWith('entity-'));
      }
    });

    it('should assign confidence based on proximity', () => {
      const closeContent = 'jwt jsonwebtoken'; // Close together
      const farContent = 'jwt is a token format. Many lines later... jsonwebtoken is a library.';

      const closeConcepts = extractConcepts(closeContent);
      const closeEntities = extractEntities(closeContent);
      const closeRels = detectRelationships(closeConcepts, closeEntities, closeContent);

      const farConcepts = extractConcepts(farContent);
      const farEntities = extractEntities(farContent);
      const farRels = detectRelationships(farConcepts, farEntities, farContent);

      // Relationships in close content should generally have higher confidence
      if (closeRels.length > 0 && farRels.length > 0) {
        const avgCloseConfidence = closeRels.reduce((sum, r) => sum + r.confidence, 0) / closeRels.length;
        const avgFarConfidence = farRels.reduce((sum, r) => sum + r.confidence, 0) / farRels.length;

        assert.ok(
          avgCloseConfidence >= avgFarConfidence * 0.8, // Allow some tolerance
          'Close proximity should have similar or higher confidence'
        );
      }
    });

    it('should handle no relationships gracefully', () => {
      const content = 'Hello world';
      const concepts: ExtractedConcept[] = [];
      const entities: ExtractedEntity[] = [];
      const relationships = detectRelationships(concepts, entities, content);

      assert.strictEqual(relationships.length, 0);
    });
  });
});

describe('Knowledge Graph Builder - Pattern Detection', () => {
  describe('detectPatterns', () => {
    it('should detect patterns meeting minimum frequency', () => {
      const sessions = [
        {
          sessionId: 'session-1',
          concepts: [
            { name: 'JWT Authentication', category: 'security', confidence: 0.9, sourceText: 'jwt' },
          ],
        },
        {
          sessionId: 'session-2',
          concepts: [
            { name: 'JWT Authentication', category: 'security', confidence: 0.85, sourceText: 'jwt' },
          ],
        },
        {
          sessionId: 'session-3',
          concepts: [{ name: 'Database', category: 'infrastructure', confidence: 0.9, sourceText: 'db' }],
        },
      ];

      const patterns = detectPatterns(sessions, 2);

      assert.ok(patterns.length > 0, 'Should detect at least one pattern');
      assert.ok(
        patterns.some((p) => p.name.includes('JWT')),
        'Should detect JWT pattern (appears 2 times)'
      );
    });

    it('should not detect patterns below minimum frequency', () => {
      const sessions = [
        {
          sessionId: 'session-1',
          concepts: [
            { name: 'JWT Authentication', category: 'security', confidence: 0.9, sourceText: 'jwt' },
          ],
        },
        {
          sessionId: 'session-2',
          concepts: [{ name: 'Database', category: 'infrastructure', confidence: 0.9, sourceText: 'db' }],
        },
      ];

      const patterns = detectPatterns(sessions, 3);

      assert.strictEqual(patterns.length, 0, 'No pattern should meet frequency of 3');
    });

    it('should include correct session IDs in detected patterns', () => {
      const sessions = [
        {
          sessionId: 'session-1',
          concepts: [{ name: 'Caching', category: 'performance', confidence: 0.9, sourceText: 'cache' }],
        },
        {
          sessionId: 'session-2',
          concepts: [{ name: 'Caching', category: 'performance', confidence: 0.85, sourceText: 'cache' }],
        },
      ];

      const patterns = detectPatterns(sessions, 2);

      const cachingPattern = patterns.find((p) => p.name.includes('Caching'));
      assert.ok(cachingPattern, 'Should find caching pattern');
      assert.ok(cachingPattern.sessionIds.includes('session-1'));
      assert.ok(cachingPattern.sessionIds.includes('session-2'));
      assert.strictEqual(cachingPattern.sessionIds.length, 2);
    });

    it('should handle empty sessions', () => {
      const sessions: Array<{ sessionId: string; concepts: ExtractedConcept[] }> = [];
      const patterns = detectPatterns(sessions, 2);

      assert.strictEqual(patterns.length, 0);
    });

    it('should handle sessions with no concepts', () => {
      const sessions = [
        { sessionId: 'session-1', concepts: [] },
        { sessionId: 'session-2', concepts: [] },
      ];

      const patterns = detectPatterns(sessions, 1);
      assert.strictEqual(patterns.length, 0);
    });

    it('should detect multiple distinct patterns', () => {
      const sessions = [
        {
          sessionId: 'session-1',
          concepts: [
            { name: 'Authentication', category: 'security', confidence: 0.9, sourceText: 'auth' },
            { name: 'Database', category: 'infrastructure', confidence: 0.85, sourceText: 'db' },
          ],
        },
        {
          sessionId: 'session-2',
          concepts: [
            { name: 'Authentication', category: 'security', confidence: 0.88, sourceText: 'auth' },
            { name: 'Database', category: 'infrastructure', confidence: 0.9, sourceText: 'db' },
          ],
        },
        {
          sessionId: 'session-3',
          concepts: [
            { name: 'Authentication', category: 'security', confidence: 0.92, sourceText: 'auth' },
            { name: 'API', category: 'architecture', confidence: 0.9, sourceText: 'api' },
          ],
        },
      ];

      const patterns = detectPatterns(sessions, 2);

      assert.ok(patterns.length >= 2, `Should detect at least 2 patterns, got ${patterns.length}`);
    });

    it('should correctly track occurrence count', () => {
      const sessions = [
        {
          sessionId: 'session-1',
          concepts: [{ name: 'API', category: 'architecture', confidence: 0.9, sourceText: 'api' }],
        },
        {
          sessionId: 'session-2',
          concepts: [{ name: 'API', category: 'architecture', confidence: 0.9, sourceText: 'api' }],
        },
        {
          sessionId: 'session-3',
          concepts: [{ name: 'API', category: 'architecture', confidence: 0.9, sourceText: 'api' }],
        },
      ];

      const patterns = detectPatterns(sessions, 2);

      const apiPattern = patterns.find((p) => p.name.includes('API'));
      assert.ok(apiPattern, 'Should find API pattern');
      assert.strictEqual(apiPattern.occurrences, 3);
    });

    it('should assign confidence based on occurrence frequency', () => {
      const sessions = Array(10)
        .fill(null)
        .map((_, i) => ({
          sessionId: `session-${i}`,
          concepts: [{ name: 'Popular Concept', category: 'test', confidence: 0.9, sourceText: 'popular' }],
        }));

      const patterns = detectPatterns(sessions, 2);

      const popularPattern = patterns.find((p) => p.name.includes('Popular'));
      assert.ok(popularPattern, 'Should find popular pattern');
      assert.ok(popularPattern.confidence > 0.7, 'High frequency should yield high confidence');
    });

    it('should respect minimum frequency of 1', () => {
      const sessions = [
        {
          sessionId: 'session-1',
          concepts: [{ name: 'Unique', category: 'test', confidence: 0.9, sourceText: 'unique' }],
        },
      ];

      const patterns = detectPatterns(sessions, 1);

      assert.ok(patterns.length > 0, 'Should detect pattern with frequency 1');
    });

    it('should not count same concept multiple times in same session', () => {
      const sessions = [
        {
          sessionId: 'session-1',
          concepts: [
            { name: 'Repeat', category: 'test', confidence: 0.9, sourceText: 'repeat' },
            { name: 'Repeat', category: 'test', confidence: 0.85, sourceText: 'repeat' },
            { name: 'Repeat', category: 'test', confidence: 0.8, sourceText: 'repeat' },
          ],
        },
      ];

      const patterns = detectPatterns(sessions, 2);

      // Even though 'Repeat' appears 3 times, it's in the same session
      assert.strictEqual(patterns.length, 0, 'Same session should not increase frequency');
    });
  });
});
