import type { HealthState } from '../hooks/useHealth';

const LABELS: Record<HealthState['status'], string> = {
  loading: 'Checking…',
  online: 'API online',
  offline: 'API offline',
};

/** Presentational health badge. Kept prop-driven so it is trivially testable. */
export function HealthIndicator({ state }: { state: HealthState }) {
  return (
    <div className="health" role="status">
      <span className={`health__dot health__dot--${state.status}`} aria-hidden="true" />
      <div className="health__body">
        <span className="health__label">{LABELS[state.status]}</span>
        {state.status === 'online' && (
          <span className="health__detail">server time {state.data.time}</span>
        )}
        {state.status === 'offline' && <span className="health__detail">{state.error}</span>}
      </div>
    </div>
  );
}
