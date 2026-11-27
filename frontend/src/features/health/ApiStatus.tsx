import { useEffect, useState } from 'react'
import { apiGet } from '../../lib/api'

type HealthResponse = { status: string; service: string; version: string }
type Status = 'checking' | 'connected' | 'unavailable'

export function ApiStatus() {
  const [status, setStatus] = useState<Status>('checking')
  const [attempt, setAttempt] = useState(0)

  useEffect(() => {
    const controller = new AbortController()
    const timeout = window.setTimeout(() => {
      controller.abort()
      setStatus('unavailable')
    }, 75_000)
    void apiGet<HealthResponse>('/health', controller.signal)
      .then((health) => {
        if (!controller.signal.aborted) {
          setStatus(health.status === 'ok' && health.service === 'matterflow-api' ? 'connected' : 'unavailable')
        }
      })
      .catch(() => {
        if (!controller.signal.aborted) setStatus('unavailable')
      })
      .finally(() => window.clearTimeout(timeout))
    return () => {
      controller.abort()
      window.clearTimeout(timeout)
    }
  }, [attempt])

  return (
    <section className="connection" aria-label="API connection">
      <p role="status" aria-live="polite">
        <span className={'status-dot ' + status} aria-hidden="true" />
        {status === 'checking' && 'Connecting to the Rust API...'}
        {status === 'connected' && 'Rust API connected'}
        {status === 'unavailable' && 'Rust API is not reachable yet. Start the backend, then retry.'}
      </p>
      {status === 'unavailable' && (
        <button onClick={() => {
          setStatus('checking')
          setAttempt((value) => value + 1)
        }}>Retry connection</button>
      )}
    </section>
  )
}
