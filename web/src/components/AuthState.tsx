/**
 * Placeholder auth-state area. Real Auth0 (Google) cookie-session identity arrives in a
 * later task; for the skeleton this just reserves the slot in the shell.
 */
export function AuthState() {
  return (
    <div className="auth">
      <span className="auth__status">Not signed in</span>
      <span className="auth__note">Authentication lands in a later slice.</span>
    </div>
  );
}
