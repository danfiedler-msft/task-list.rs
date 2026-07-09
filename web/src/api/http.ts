/** Error thrown by {@link apiFetch} when the API responds with a non-2xx status. */
export class ApiError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

/**
 * Base fetch wrapper for same-origin `/api/*` calls. Sends/accepts JSON, and turns non-2xx
 * responses into a typed {@link ApiError}. The generated client types (ApiClient.generated)
 * describe the shapes; this wrapper carries them over the wire.
 */
export async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    ...init,
    headers: {
      Accept: 'application/json',
      ...init?.headers,
    },
  });

  if (!response.ok) {
    throw new ApiError(response.status, `${init?.method ?? 'GET'} ${path} failed (${response.status})`);
  }

  return (await response.json()) as T;
}
