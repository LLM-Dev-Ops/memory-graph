/**
 * Execution Unit Orchestrator
 *
 * The central enforcement point for the Foundational Execution Unit protocol.
 * This orchestrator:
 *
 * 1. Validates execution_id and parent_span_id (rejects if missing)
 * 2. Creates the repo-level span (type: "repo", repo_name: "memory-graph")
 * 3. Invokes each requested agent via its adapter
 * 4. Collects agent-level spans with attached artifacts
 * 5. Enforces: no success without agent spans (requirement 6)
 * 6. On failure: still returns all emitted spans (requirement 7)
 * 7. Returns a complete ExecutionResponse with the span tree
 */

import type {
  ExecutionRequest,
  ExecutionResponse,
  ExecutionErrorResponse,
  AgentSpan,
  RepoSpan,
  ExecutionSummary,
} from './types.js';
import { ExecutionRequestSchema, REPO_NAME } from './types.js';
import { executeAndBuildSpan, createAgentAdapter } from './agent-adapter.js';
import type { AdapterConfig, AgentExecutionResult } from './agent-adapter.js';

// ============================================================================
// ORCHESTRATOR
// ============================================================================

/**
 * Configuration for the execution unit orchestrator.
 */
export interface OrchestratorConfig {
  /** Override adapter configs per agent_id */
  adapterConfigs?: Record<string, AdapterConfig>;
}

/**
 * Result type: either a valid ExecutionResponse or an error before execution started.
 */
export type OrchestratorResult =
  | { ok: true; response: ExecutionResponse }
  | { ok: false; error: ExecutionErrorResponse };

/**
 * ExecutionUnitOrchestrator enforces all FEU invariants.
 *
 * Usage:
 * ```typescript
 * const orchestrator = new ExecutionUnitOrchestrator();
 * const result = await orchestrator.execute(request);
 * if (result.ok) {
 *   // result.response contains the full span tree
 * } else {
 *   // result.error describes why the request was rejected
 * }
 * ```
 */
export class ExecutionUnitOrchestrator {
  private readonly config: OrchestratorConfig;

  constructor(config?: OrchestratorConfig) {
    this.config = config ?? {};
  }

  /**
   * Execute the FEU request. This is the main entry point.
   *
   * ENFORCEMENT:
   * - Rejects if execution_id or parent_span_id is missing (requirement 2)
   * - Creates repo-level span immediately (requirement 3)
   * - Creates one agent span per agent (requirement 4)
   * - Never returns success with zero agent spans (requirement 6)
   * - Always returns spans even on failure (requirement 7)
   * - Forbids returning success if any agent ran without a span (requirement 9)
   */
  async execute(request: unknown): Promise<OrchestratorResult> {
    // ── Step 1: Validate request ──────────────────────────────────────
    const parseResult = ExecutionRequestSchema.safeParse(request);

    if (!parseResult.success) {
      const missingFields = parseResult.error.issues.map((i) => i.path.join('.')).join(', ');
      return {
        ok: false,
        error: {
          error_code: 'VALIDATION_ERROR',
          message: `Invalid execution request. Issues: ${missingFields}`,
          details: { issues: parseResult.error.issues },
        },
      };
    }

    const validRequest: ExecutionRequest = parseResult.data;
    const { execution_id, parent_span_id, agents, options } = validRequest;

    // ── Step 2: Create repo-level span ────────────────────────────────
    const repoSpanId = crypto.randomUUID();
    const repoStartedAt = new Date().toISOString();
    const repoStartTime = Date.now();
    const agentSpans: AgentSpan[] = [];

    // ── Step 3: Execute each agent ────────────────────────────────────
    for (const agentReq of agents) {
      let adapter;
      try {
        const adapterConfig = this.config.adapterConfigs?.[agentReq.agent_id];
        adapter = createAgentAdapter(agentReq.agent_id, adapterConfig);
      } catch (err) {
        // Unknown agent -- emit an error span for it
        agentSpans.push({
          span_id: crypto.randomUUID(),
          parent_span_id: repoSpanId,
          span_type: 'agent',
          agent_id: agentReq.agent_id,
          agent_version: '0.0.0',
          status: 'error',
          started_at: new Date().toISOString(),
          ended_at: new Date().toISOString(),
          duration_ms: 0,
          error: {
            error_code: 'UNKNOWN_AGENT',
            message: err instanceof Error ? err.message : String(err),
          },
          artifacts: [],
        });

        if (options?.fail_fast) break;
        continue;
      }

      const agentSpan = await executeAndBuildSpan(
        adapter,
        agentReq.input,
        execution_id,
        repoSpanId,
      );
      agentSpans.push(agentSpan);

      // Fail-fast: stop on first error if configured
      if (options?.fail_fast && agentSpan.status === 'error') {
        break;
      }
    }

    // ── Step 4: Enforcement checks ────────────────────────────────────

    // INVARIANT: If no agent spans were emitted, execution is INVALID.
    // This should never happen since we always emit spans (even error ones),
    // but we enforce it defensively.
    if (agentSpans.length === 0) {
      return {
        ok: false,
        error: {
          execution_id,
          error_code: 'INVARIANT_VIOLATION',
          message: 'No agent-level spans were emitted. Execution is invalid.',
        },
      };
    }

    // ── Step 5: Assemble repo span ────────────────────────────────────
    const repoEndedAt = new Date().toISOString();
    const repoDurationMs = Date.now() - repoStartTime;

    const allSucceeded = agentSpans.every((s) => s.status === 'ok');

    const repoSpan: RepoSpan = {
      span_id: repoSpanId,
      parent_span_id,
      execution_id,
      span_type: 'repo',
      repo_name: REPO_NAME,
      status: allSucceeded ? 'ok' : 'error',
      started_at: repoStartedAt,
      ended_at: repoEndedAt,
      duration_ms: repoDurationMs,
      agent_spans: agentSpans,
    };

    // ── Step 6: Build summary ─────────────────────────────────────────
    const succeeded = agentSpans.filter((s) => s.status === 'ok').length;
    const failed = agentSpans.filter((s) => s.status !== 'ok').length;
    const totalArtifacts = agentSpans.reduce((sum, s) => sum + s.artifacts.length, 0);

    const summary: ExecutionSummary = {
      total_agents: agentSpans.length,
      succeeded,
      failed,
      total_artifacts: totalArtifacts,
      total_duration_ms: repoDurationMs,
    };

    // ── Step 7: Return response ───────────────────────────────────────
    return {
      ok: true,
      response: {
        execution_id,
        repo_span: repoSpan,
        success: allSucceeded,
        summary,
      },
    };
  }
}

/**
 * Create and execute in one call (convenience function).
 */
export async function executeUnit(
  request: unknown,
  config?: OrchestratorConfig,
): Promise<OrchestratorResult> {
  const orchestrator = new ExecutionUnitOrchestrator(config);
  return orchestrator.execute(request);
}
