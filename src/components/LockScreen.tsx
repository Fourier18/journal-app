import { useState } from "react";
import { createVault, unlockVault } from "../lib/commands";
import "./LockScreen.css";

interface Props {
  mode: "create" | "unlock";
  onSuccess: () => void;
}

export default function LockScreen({ mode, onSuccess }: Props) {
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const isCreate = mode === "create";

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);

    if (isCreate && password !== confirm) {
      setError("Passwords don't match.");
      return;
    }
    if (password.length < 8) {
      setError("Password must be at least 8 characters.");
      return;
    }

    setBusy(true);
    try {
      if (isCreate) {
        await createVault(password);
      } else {
        await unlockVault(password);
      }
      onSuccess();
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg.includes("Wrong password") ? "Incorrect password." : msg);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="lock-screen">
      <div className="lock-card">
        <div className="lock-icon">📓</div>
        <h1 className="lock-title">
          {isCreate ? "Create Your Journal" : "Welcome Back"}
        </h1>
        <p className="lock-subtitle">
          {isCreate
            ? "Choose a master password. You'll need it every time you open your journal."
            : "Enter your master password to unlock your journal."}
        </p>

        <form onSubmit={handleSubmit} className="lock-form">
          <label className="lock-label">
            Password
            <input
              type="password"
              className="lock-input"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Enter password"
              autoFocus
              disabled={busy}
            />
          </label>

          {isCreate && (
            <label className="lock-label">
              Confirm password
              <input
                type="password"
                className="lock-input"
                value={confirm}
                onChange={(e) => setConfirm(e.target.value)}
                placeholder="Confirm password"
                disabled={busy}
              />
            </label>
          )}

          {error && <p className="lock-error">{error}</p>}

          <button type="submit" className="lock-btn" disabled={busy || !password}>
            {busy ? "Please wait…" : isCreate ? "Create Journal" : "Unlock"}
          </button>
        </form>
      </div>
    </div>
  );
}
