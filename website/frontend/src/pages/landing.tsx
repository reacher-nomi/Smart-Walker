import { Link } from "react-router-dom";

export default function Landing() {
  return (
    <div className="page">
      <div className="card">
        <h1>MedHealth Monitor</h1>
        <p className="muted">
          Real-time visualization of health sensor readings (HR, SpO2, temperature).
        </p>

        <div className="row">
          <Link className="btn" to="/auth">Login / Sign up</Link>
          <Link className="btn btn-ghost" to="/dashboard">Open dashboard</Link>
        </div>

        <div className="note">
          <div className="note-title">Backend</div>
          <div className="note-text">
            Dashboard expects <code>GET /heartrate</code> on your Pi backend (similar to your Flask example). [file:8]
          </div>
        </div>
      </div>
    </div>
  );
}
