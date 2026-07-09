import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { HealthIndicator } from './HealthIndicator';

describe('HealthIndicator', () => {
  it('shows the online label and server time', () => {
    render(<HealthIndicator state={{ status: 'online', data: { status: 'ok', time: '2026-07-06T00:00:00Z' } }} />);

    expect(screen.getByText('API online')).toBeInTheDocument();
    expect(screen.getByText(/2026-07-06T00:00:00Z/)).toBeInTheDocument();
  });

  it('shows the offline label and error', () => {
    render(<HealthIndicator state={{ status: 'offline', error: 'boom' }} />);

    expect(screen.getByText('API offline')).toBeInTheDocument();
    expect(screen.getByText('boom')).toBeInTheDocument();
  });

  it('shows a checking label while loading', () => {
    render(<HealthIndicator state={{ status: 'loading' }} />);

    expect(screen.getByText('Checking…')).toBeInTheDocument();
  });
});
