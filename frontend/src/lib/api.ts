const baseUrl = (import.meta.env.VITE_API_BASE_URL ?? "").replace(/\/+$/, "");

export class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

type Options = {
  method?: string;
  body?: unknown;
  token?: string;
  signal?: AbortSignal;
};

export async function apiRequest<T>(
  path: string,
  options: Options = {},
): Promise<T> {
  const headers: Record<string, string> = { Accept: "application/json" };
  if (options.body !== undefined) headers["Content-Type"] = "application/json";
  if (options.token) headers.Authorization = "Bearer " + options.token;
  const timeout = AbortSignal.timeout(75_000);
  const response = await fetch(baseUrl + "/api/v1" + path, {
    method: options.method ?? "GET",
    headers,
    body: options.body === undefined ? undefined : JSON.stringify(options.body),
    signal: options.signal
      ? AbortSignal.any([options.signal, timeout])
      : timeout,
    cache: "no-store",
  });
  if (!response.ok) {
    const body = await response.json().catch(() => null);
    throw new ApiError(
      response.status,
      body?.error?.message ??
        "The request could not be completed. Please try again.",
    );
  }
  if (response.status === 204) return undefined as T;
  return response.json() as Promise<T>;
}

export function apiGet<T>(
  path: string,
  signal?: AbortSignal,
  token?: string,
): Promise<T> {
  return apiRequest<T>(path, { signal, token });
}
