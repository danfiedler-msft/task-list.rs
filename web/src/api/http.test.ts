import { afterEach, describe, expect, it, vi } from 'vitest';

import { ApiError, apiFetch } from './http';

describe('apiFetch', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('parses a JSON body on success', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response(JSON.stringify({ status: 'ok', time: 't' }), { status: 200 })),
    );

    const body = await apiFetch<{ status: string; time: string }>('/api/health');

    expect(body).toEqual({ status: 'ok', time: 't' });
  });

  it('throws a typed ApiError with the status on failure', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response('nope', { status: 503 })),
    );

    await expect(apiFetch('/api/health')).rejects.toMatchObject({
      name: 'ApiError',
      status: 503,
    });
    await expect(apiFetch('/api/health')).rejects.toBeInstanceOf(ApiError);
  });
});
