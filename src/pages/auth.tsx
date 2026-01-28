import { useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { setAuth } from "../lib/auth";
import { apiPostJson } from "../lib/api";

/**
 * This is a combined Login+Signup UI.
 * For now it's "frontend-only" (stores email in localStorage).
 * Later: replace handleSubmit() with Rust backend calls.
 */
export default function Auth() {
  const nav = useNavigate();

  const [mode, setMode] = useState<"login" | "signup">("login");
  const isSignup = useMemo(() => mode === "signup", [mode]);

  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(e: React.FormEvent) {
  e.preventDefault();
  setError(null);

  if (!email.includes("@")) return setError("Please enter a valid email.");
  if (password.length < 8) return setError("Password must be at least 8 characters.");
  if (isSignup && password !== confirm) return setError("Passwords do not match.");

  setBusy(true);
  try {
    let response: any;
    if (isSignup) {
      await apiPostJson("/auth/signup", { email, password });
      response = await apiPostJson("/auth/login", { email, password }); // auto-login
    } else {
      response = await apiPostJson("/auth/login", { email, password });
    }

    // Store auth data with token
    if (response && response.token) {
      setAuth(email, response.token);
    } else {
      setAuth(email);
    }

    nav("/dashboard");
  } catch (e: any) {
    setError(e?.message ?? "Auth failed.");
  } finally {
    setBusy(false);
  }
}


  return (
    <div className="page">
      <div className="card">
        <h2>{isSignup ? "Create account" : "Login"}</h2>

        <div className="tabs">
          <button
            className={mode === "login" ? "tab tab-active" : "tab"}
            onClick={() => setMode("login")}
            type="button"
          >
            Login
          </button>
          <button
            className={mode === "signup" ? "tab tab-active" : "tab"}
            onClick={() => setMode("signup")}
            type="button"
          >
            Sign up
          </button>
        </div>

        <form className="form" onSubmit={handleSubmit}>
          <label>
            Email
            <input
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              type="email"
              autoComplete="email"
              required
            />
          </label>

          <label>
            Password
            <input
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              type="password"
              autoComplete={isSignup ? "new-password" : "current-password"}
              required
              minLength={8}
            />
          </label>

          {isSignup && (
            <label>
              Confirm password
              <input
                value={confirm}
                onChange={(e) => setConfirm(e.target.value)}
                type="password"
                autoComplete="new-password"
                required
              />
            </label>
          )}

          {error && <div className="alert">{error}</div>}

          <button className="btn" type="submit" disabled={busy}>
            {busy ? "Please wait..." : isSignup ? "Sign up" : "Login"}
          </button>
        </form>

        <p className="muted">
          This auth is temporary until Rust backend auth endpoints exist.
        </p>
      </div>
    </div>
  );
}
