import { HealthIndicator } from './components/HealthIndicator';
import { AuthState } from './components/AuthState';
import { useHealth } from './hooks/useHealth';

export default function App() {
  const health = useHealth();

  return (
    <main className="app">
      <header className="app__header">
        <h1 className="app__title">task-list.rs</h1>
        <p className="app__subtitle">Clean-architecture starter · walking skeleton</p>
      </header>

      <section className="card" aria-label="Service status">
        <HealthIndicator state={health} />
      </section>

      <section className="card" aria-label="Authentication">
        <AuthState />
      </section>
    </main>
  );
}
