import { useEffect, useState, type ReactNode } from "react";
import { apiGet } from "../../lib/api";

type HealthResponse = { status: string; service: string; version: string };
type Props = { children?: (ready: boolean) => ReactNode };

export function ApiStatus({ children }: Props) {
  const [ready, setReady] = useState(false);
  const [takingLonger, setTakingLonger] = useState(false);

  useEffect(() => {
    const controller = new AbortController();
    let retry: ReturnType<typeof window.setTimeout> | undefined;
    const started = Date.now();
    const progress = window.setTimeout(() => setTakingLonger(true), 60_000);

    async function check() {
      try {
        const health = await apiGet<HealthResponse>(
          "/ready",
          AbortSignal.any([controller.signal, AbortSignal.timeout(20_000)]),
        );
        if (controller.signal.aborted) return;
        if (health.status === "ready" && health.service === "matterflow-api") {
          window.clearTimeout(progress);
          setReady(true);
          return;
        }
      } catch {
        // Cold starts, temporary database failures, and network errors are retried.
      }
      if (!controller.signal.aborted) {
        retry = window.setTimeout(
          check,
          Date.now() - started < 120_000 ? 3_000 : 15_000,
        );
      }
    }

    void check();
    return () => {
      controller.abort();
      window.clearTimeout(progress);
      window.clearTimeout(retry);
    };
  }, []);

  return (
    <>
      {children?.(ready)}
      <section className="connection" aria-label="API connection">
        <p role="status" aria-live="polite">
          <span
            className={"status-dot " + (ready ? "connected" : "checking")}
            aria-hidden="true"
          />
          {ready
            ? "Demo service is ready"
            : takingLonger
              ? "The demo is still getting ready. We are reconnecting automatically; you can leave this page open."
              : "Preparing your demo... Connecting automatically. This may take a minute after inactivity."}
        </p>
      </section>
    </>
  );
}
