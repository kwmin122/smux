import { useState } from 'react'

interface GitInfoProps {
  branch: string
  filesChanged: number
  onAutoCommit?: () => void
}

/**
 * Replaces common secret patterns in text with [REDACTED].
 *
 * Covers:
 * - API keys: sk-..., ghp_..., AKIA...
 * - Bearer tokens: Bearer <token>
 * - Generic key=value where key contains secret, password, token, key, api_key
 */
export function redactSecrets(text: string): string {
  let result = text

  // Bearer tokens
  result = result.replace(/Bearer\s+[A-Za-z0-9\-._~+/]+=*/g, 'Bearer [REDACTED]')

  // OpenAI / Stripe style: sk-... (at least 20 chars)
  result = result.replace(/\bsk-[A-Za-z0-9]{20,}\b/g, '[REDACTED]')

  // GitHub personal access tokens: ghp_...
  result = result.replace(/\bghp_[A-Za-z0-9]{20,}\b/g, '[REDACTED]')

  // GitHub OAuth tokens: gho_...
  result = result.replace(/\bgho_[A-Za-z0-9]{20,}\b/g, '[REDACTED]')

  // GitHub app tokens: ghs_..., ghr_...
  result = result.replace(/\bghs_[A-Za-z0-9]{20,}\b/g, '[REDACTED]')
  result = result.replace(/\bghr_[A-Za-z0-9]{20,}\b/g, '[REDACTED]')

  // AWS access key IDs: AKIA...
  result = result.replace(/\bAKIA[A-Z0-9]{16}\b/g, '[REDACTED]')

  // Anthropic API keys: sk-ant-api03-...
  result = result.replace(/\bsk-ant-api\d{2}-[A-Za-z0-9\-_]{20,}\b/g, '[REDACTED]')

  // Slack tokens: xoxb-, xoxp-, xoxs-
  result = result.replace(/\bxox[bps]-[A-Za-z0-9\-]{20,}\b/g, '[REDACTED]')

  // npm tokens: npm_...
  result = result.replace(/\bnpm_[A-Za-z0-9]{20,}\b/g, '[REDACTED]')

  // PyPI tokens: pypi-...
  result = result.replace(/\bpypi-[A-Za-z0-9]{20,}\b/g, '[REDACTED]')

  // Hugging Face tokens: hf_...
  result = result.replace(/\bhf_[A-Za-z0-9]{20,}\b/g, '[REDACTED]')

  // SSH private keys
  result = result.replace(/-----BEGIN[A-Z ]*PRIVATE KEY-----[\s\S]*?-----END[A-Z ]*PRIVATE KEY-----/g, '[REDACTED SSH KEY]')

  // Basic auth headers: Basic <base64>
  result = result.replace(/Basic\s+[A-Za-z0-9+/]{16,}={0,2}/g, 'Basic [REDACTED]')

  // Connection strings with credentials: postgres://user:pass@..., mongodb://...
  result = result.replace(/((?:postgres|postgresql|mysql|mongodb|redis|amqp):\/\/[^:]+:)[^\s@]+(@)/g, '$1[REDACTED]$2')

  // Generic key=value patterns where key contains sensitive words
  result = result.replace(
    /\b([A-Za-z_]*(?:secret|password|passwd|token|api_key|apikey|auth_key|authkey|access_key|private_key)[A-Za-z_]*)\s*[=:]\s*["']?([^\s"',;]{4,})["']?/gi,
    '$1=[REDACTED]'
  )

  return result
}

/**
 * Compact git status display for the sidebar.
 * Shows branch name, changed file count badge, and optional auto-commit button.
 */
export function GitInfo({ branch, filesChanged, onAutoCommit }: GitInfoProps) {
  const [redactionEnabled] = useState(true)

  return (
    <div
      className="flex items-center gap-2 px-2 font-mono text-[10px] text-on-surface-variant select-none"
      style={{ height: 40 }}
    >
      {/* Branch icon + name */}
      <div className="flex items-center gap-1 min-w-0 flex-1">
        <span className="material-symbols-outlined text-[14px] text-outline shrink-0" aria-hidden="true">
          fork_right
        </span>
        <span className="truncate" title={branch}>
          {branch}
        </span>
      </div>

      {/* Changed files badge */}
      <span
        className={`inline-flex items-center justify-center rounded-full px-1.5 min-w-[18px] h-[16px] text-[9px] font-bold leading-none ${
          filesChanged > 0
            ? 'bg-yellow-400/20 text-yellow-400'
            : 'bg-outline-variant/20 text-outline'
        }`}
        title={`${filesChanged} file${filesChanged !== 1 ? 's' : ''} changed`}
      >
        {filesChanged}
      </span>

      {/* Auto-commit button */}
      {filesChanged > 0 && onAutoCommit && (
        <button
          type="button"
          onClick={onAutoCommit}
          className="px-1.5 py-0.5 rounded border border-outline-variant/30 text-[9px] text-primary hover:bg-primary/10 transition-colors cursor-pointer whitespace-nowrap"
        >
          Auto-commit
        </button>
      )}

      {/* Secret redaction lock indicator */}
      {redactionEnabled && (
        <span
          className="material-symbols-outlined text-[12px] text-secondary shrink-0"
          title="Secret redaction enabled"
          aria-label="Secret redaction enabled"
        >
          lock
        </span>
      )}
    </div>
  )
}
