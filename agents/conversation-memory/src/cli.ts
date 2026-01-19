/**
 * Conversation Memory Agent - CLI Interface
 *
 * Provides CLI-invokable endpoints for:
 * - inspect: View a specific DecisionEvent by execution_ref
 * - retrieve: Query DecisionEvents by criteria
 * - replay: Re-execute a previous capture operation
 *
 * Usage:
 *   memory-graph agent conversation-memory inspect <execution_ref>
 *   memory-graph agent conversation-memory retrieve --session-id <id> --limit 10
 *   memory-graph agent conversation-memory replay <execution_ref>
 */

import { RuVectorClient } from './ruvector-client.js';
import { createAgent } from './agent.js';
import type { DecisionEvent, ConversationCaptureInput } from './types.js';

/**
 * CLI output format options
 */
export type OutputFormat = 'json' | 'table' | 'yaml';

/**
 * CLI command result
 */
export interface CLIResult {
  success: boolean;
  data?: unknown;
  error?: string;
}

/**
 * Inspect command options
 */
export interface InspectOptions {
  executionRef: string;
  format?: OutputFormat;
  verbose?: boolean;
}

/**
 * Retrieve command options
 */
export interface RetrieveOptions {
  sessionId?: string;
  conversationId?: string;
  fromTimestamp?: string;
  toTimestamp?: string;
  limit?: number;
  format?: OutputFormat;
}

/**
 * Replay command options
 */
export interface ReplayOptions {
  executionRef: string;
  dryRun?: boolean;
  format?: OutputFormat;
}

/**
 * CLI handler for the Conversation Memory Agent
 */
export class ConversationMemoryCLI {
  private readonly ruvectorClient: RuVectorClient;

  constructor(ruvectorClient?: RuVectorClient) {
    this.ruvectorClient = ruvectorClient ?? new RuVectorClient();
  }

  /**
   * Inspect a DecisionEvent by execution_ref
   *
   * Usage: memory-graph agent conversation-memory inspect <execution_ref>
   */
  async inspect(options: InspectOptions): Promise<CLIResult> {
    try {
      const event = await this.ruvectorClient.retrieveDecisionEvent(options.executionRef);

      if (!event) {
        return {
          success: false,
          error: `DecisionEvent not found: ${options.executionRef}`,
        };
      }

      const output = options.verbose
        ? event
        : this.summarizeDecisionEvent(event);

      return {
        success: true,
        data: this.formatOutput(output, options.format ?? 'json'),
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Retrieve DecisionEvents by criteria
   *
   * Usage: memory-graph agent conversation-memory retrieve --session-id <id>
   */
  async retrieve(options: RetrieveOptions): Promise<CLIResult> {
    try {
      const events = await this.ruvectorClient.queryDecisionEvents({
        agent_id: 'conversation-memory-agent',
        decision_type: 'conversation_capture',
        session_id: options.sessionId,
        from_timestamp: options.fromTimestamp,
        to_timestamp: options.toTimestamp,
        limit: options.limit ?? 100,
      });

      const output = events.map((event) => this.summarizeDecisionEvent(event));

      return {
        success: true,
        data: this.formatOutput(
          {
            count: events.length,
            events: output,
          },
          options.format ?? 'json'
        ),
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Replay a previous capture operation
   *
   * Usage: memory-graph agent conversation-memory replay <execution_ref>
   */
  async replay(options: ReplayOptions): Promise<CLIResult> {
    try {
      // Retrieve the original DecisionEvent
      const originalEvent = await this.ruvectorClient.retrieveDecisionEvent(
        options.executionRef
      );

      if (!originalEvent) {
        return {
          success: false,
          error: `Original DecisionEvent not found: ${options.executionRef}`,
        };
      }

      // NOTE: In production, we would store the original input in ruvector-service
      // For now, we demonstrate the pattern
      if (options.dryRun) {
        return {
          success: true,
          data: this.formatOutput(
            {
              operation: 'replay',
              mode: 'dry_run',
              original_execution_ref: options.executionRef,
              original_timestamp: originalEvent.timestamp,
              message: 'Would re-execute with original input',
            },
            options.format ?? 'json'
          ),
        };
      }

      // In production: fetch original input and re-execute
      // const originalInput = await this.ruvectorClient.getOriginalInput(options.executionRef);
      // const agent = createAgent();
      // const result = await agent.execute(originalInput);

      return {
        success: true,
        data: this.formatOutput(
          {
            operation: 'replay',
            original_execution_ref: options.executionRef,
            message:
              'Replay would fetch original input from ruvector-service and re-execute',
          },
          options.format ?? 'json'
        ),
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Execute a new capture operation from CLI
   *
   * Usage: memory-graph agent conversation-memory capture --input <file>
   */
  async capture(input: ConversationCaptureInput): Promise<CLIResult> {
    try {
      const agent = createAgent();
      const result = await agent.execute(input);

      if (result.success) {
        return {
          success: true,
          data: {
            execution_ref: result.decisionEvent.execution_ref,
            turn_count: result.output.turn_count,
            nodes_created: result.output.nodes_created.length,
            edges_created: result.output.edges_created.length,
          },
        };
      }

      return {
        success: false,
        error: result.error.message,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Get agent health status
   *
   * Usage: memory-graph agent conversation-memory health
   */
  async health(): Promise<CLIResult> {
    try {
      const ruvectorHealthy = await this.ruvectorClient.healthCheck();

      return {
        success: true,
        data: {
          agent_id: 'conversation-memory-agent',
          version: '1.0.0',
          classification: 'MEMORY_WRITE',
          decision_type: 'conversation_capture',
          status: 'healthy',
          dependencies: {
            ruvector_service: ruvectorHealthy ? 'connected' : 'disconnected',
          },
          timestamp: new Date().toISOString(),
        },
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Summarize a DecisionEvent for display
   */
  private summarizeDecisionEvent(event: DecisionEvent): Record<string, unknown> {
    return {
      execution_ref: event.execution_ref,
      agent_id: event.agent_id,
      agent_version: event.agent_version,
      decision_type: event.decision_type,
      timestamp: event.timestamp,
      confidence: event.confidence,
      turn_count: event.outputs.turn_count,
      nodes_created: event.outputs.nodes_created.length,
      edges_created: event.outputs.edges_created.length,
      session_id: event.outputs.session_id,
      conversation_id: event.outputs.conversation_id,
    };
  }

  /**
   * Format output based on requested format
   */
  private formatOutput(data: unknown, format: OutputFormat): unknown {
    switch (format) {
      case 'table':
        // For CLI, tables would be formatted by the CLI framework
        return data;
      case 'yaml':
        // Simple YAML-like formatting
        return this.toYamlLike(data);
      case 'json':
      default:
        return data;
    }
  }

  /**
   * Convert to YAML-like string format
   */
  private toYamlLike(data: unknown, indent = 0): string {
    const prefix = '  '.repeat(indent);

    if (data === null || data === undefined) {
      return 'null';
    }

    if (typeof data === 'string' || typeof data === 'number' || typeof data === 'boolean') {
      return String(data);
    }

    if (Array.isArray(data)) {
      if (data.length === 0) return '[]';
      return data.map((item) => `${prefix}- ${this.toYamlLike(item, indent + 1)}`).join('\n');
    }

    if (typeof data === 'object') {
      const entries = Object.entries(data);
      if (entries.length === 0) return '{}';
      return entries
        .map(([key, value]) => {
          const valueStr = this.toYamlLike(value, indent + 1);
          if (typeof value === 'object' && value !== null) {
            return `${prefix}${key}:\n${valueStr}`;
          }
          return `${prefix}${key}: ${valueStr}`;
        })
        .join('\n');
    }

    return String(data);
  }
}

/**
 * Create CLI handler instance
 */
export function createCLI(): ConversationMemoryCLI {
  return new ConversationMemoryCLI();
}

/**
 * CLI command definitions for integration with main CLI
 */
export const CLI_COMMANDS = {
  name: 'conversation-memory',
  description: 'Conversation Memory Agent - MEMORY WRITE classification',
  subcommands: [
    {
      name: 'inspect',
      description: 'Inspect a DecisionEvent by execution reference',
      args: [{ name: 'execution_ref', required: true, description: 'Execution reference UUID' }],
      flags: [
        { name: 'format', short: 'f', description: 'Output format (json, table, yaml)', default: 'json' },
        { name: 'verbose', short: 'v', description: 'Show full DecisionEvent details', type: 'boolean' },
      ],
    },
    {
      name: 'retrieve',
      description: 'Query DecisionEvents by criteria',
      flags: [
        { name: 'session-id', short: 's', description: 'Filter by session ID' },
        { name: 'conversation-id', short: 'c', description: 'Filter by conversation ID' },
        { name: 'from', description: 'Start timestamp (ISO 8601)' },
        { name: 'to', description: 'End timestamp (ISO 8601)' },
        { name: 'limit', short: 'l', description: 'Maximum results', default: '100' },
        { name: 'format', short: 'f', description: 'Output format (json, table, yaml)', default: 'json' },
      ],
    },
    {
      name: 'replay',
      description: 'Re-execute a previous capture operation',
      args: [{ name: 'execution_ref', required: true, description: 'Execution reference to replay' }],
      flags: [
        { name: 'dry-run', description: 'Preview replay without executing', type: 'boolean' },
        { name: 'format', short: 'f', description: 'Output format (json, table, yaml)', default: 'json' },
      ],
    },
    {
      name: 'capture',
      description: 'Capture a new conversation',
      flags: [
        { name: 'input', short: 'i', required: true, description: 'Input file path (JSON)' },
        { name: 'format', short: 'f', description: 'Output format (json, table, yaml)', default: 'json' },
      ],
    },
    {
      name: 'health',
      description: 'Check agent health status',
      flags: [],
    },
  ],
} as const;
