import { useCallback, useEffect, useState } from "react";
import { ApiError, apiRequest } from "../../lib/api";

export type Role = "staff" | "reviewer";
export type DemoSession = {
  workspace_id: string;
  user_id: string;
  display_name: string;
  role: Role;
  expires_at: string;
};
type CreatedSession = { token: string; session: DemoSession };
const storageKey = "matterflow-demo-token";

function readToken(): string | null {
  try {
    return sessionStorage.getItem(storageKey);
  } catch {
    return null;
  }
}
function storeToken(token: string | null) {
  try {
    if (token) sessionStorage.setItem(storageKey, token);
    else sessionStorage.removeItem(storageKey);
  } catch {
    /* The current in-memory demo still works when browser storage is disabled. */
  }
}

export function useDemoSession() {
  const [initialToken] = useState(readToken);
  const [token, setToken] = useState<string | null>(initialToken);
  const [session, setSession] = useState<DemoSession | null>(null);
  const [restoring, setRestoring] = useState(Boolean(initialToken));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const expire = useCallback(() => {
    storeToken(null);
    setToken(null);
    setSession(null);
    setError(
      "Your demo session has expired. You can start a fresh demo below.",
    );
  }, []);

  useEffect(() => {
    if (!initialToken) return;
    const controller = new AbortController();
    void apiRequest<DemoSession>("/demo/session", {
      token: initialToken,
      signal: controller.signal,
    })
      .then((session) => {
        if (!controller.signal.aborted) setSession(session);
      })
      .catch((failure: unknown) => {
        if (controller.signal.aborted) return;
        if (failure instanceof ApiError && failure.status === 401) expire();
        else setError("Could not restore the demo. Please try again shortly.");
      })
      .finally(() => {
        if (!controller.signal.aborted) setRestoring(false);
      });
    return () => controller.abort();
  }, [initialToken, expire]);

  async function enter(role: Role) {
    setBusy(true);
    setError("");
    try {
      const created = await apiRequest<CreatedSession>("/demo/sessions", {
        method: "POST",
        body: { role },
      });
      storeToken(created.token);
      setToken(created.token);
      setSession(created.session);
    } catch (failure) {
      setError(
        failure instanceof Error
          ? failure.message
          : "Could not start the demo.",
      );
    } finally {
      setBusy(false);
    }
  }

  async function switchRole(role: Role) {
    if (!token) return;
    setBusy(true);
    setError("");
    try {
      setSession(
        await apiRequest<DemoSession>("/demo/session", {
          method: "PATCH",
          token,
          body: { role },
        }),
      );
    } catch (failure) {
      if (failure instanceof ApiError && failure.status === 401) expire();
      else
        setError(
          failure instanceof Error
            ? failure.message
            : "Could not switch roles.",
        );
    } finally {
      setBusy(false);
    }
  }

  async function end() {
    if (!token) return;
    setBusy(true);
    setError("");
    try {
      await apiRequest<void>("/demo/session", { method: "DELETE", token });
      storeToken(null);
      setToken(null);
      setSession(null);
    } catch (failure) {
      if (failure instanceof ApiError && failure.status === 401) expire();
      else
        setError(
          failure instanceof Error
            ? failure.message
            : "Could not end the demo.",
        );
    } finally {
      setBusy(false);
    }
  }

  return {
    token,
    session,
    restoring,
    busy,
    error,
    enter,
    switchRole,
    end,
    expire,
  };
}
