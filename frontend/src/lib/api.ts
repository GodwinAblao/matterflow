const baseUrl = (import.meta.env.VITE_API_BASE_URL ?? '').replace(/\/+$/, '')

export async function apiGet<T>(path: string, signal?: AbortSignal): Promise<T> {
  const response = await fetch(baseUrl + '/api/v1' + path, {
    headers: { Accept: 'application/json' },
    signal,
  })
  if (!response.ok) {
    throw new Error('API request failed (' + response.status + ')')
  }
  return response.json() as Promise<T>
}
