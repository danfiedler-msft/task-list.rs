import { useEffect, useState } from 'react';

import { api, type HealthResponse } from '../api/client';

/** Discriminated health state consumed by the UI. */
export type HealthState =
  | { status: 'loading' }
  | { status: 'online'; data: HealthResponse }
  | { status: 'offline'; error: string };

/** Probe `/api/health` once on mount and expose the result as a discriminated state. */
export function useHealth(): HealthState {
  const [state, setState] = useState<HealthState>({ status: 'loading' });

  useEffect(() => {
    let active = true;

    api
      .getHealth()
      .then((data) => {
        if (active) {
          setState({ status: 'online', data });
        }
      })
      .catch((error: unknown) => {
        if (active) {
          const message = error instanceof Error ? error.message : 'unreachable';
          setState({ status: 'offline', error: message });
        }
      });

    return () => {
      active = false;
    };
  }, []);

  return state;
}
