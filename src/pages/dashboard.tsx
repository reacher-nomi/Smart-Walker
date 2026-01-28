import { useEffect, useMemo, useRef, useState } from "react";
import * as d3 from "d3";
import { apiGetJson } from "../lib/api";
import { clearAuth, getAuth, getAuthToken } from "../lib/auth";
import { useNavigate } from "react-router-dom";

type BackendLatest = {
  heartRate?: number;
  spo2?: number;
  temperature?: number;
  hr?: number;
  temp?: number;
  time?: number;
};

type Reading = {
  t: number;
  heartRate: number;
  spo2: number;
  temperature: number;
};

type MlAlert = {
  level: string;
  message: string;
  details: any;
};

export default function Dashboard() {
  const nav = useNavigate();
  const auth = getAuth();

  const [connected, setConnected] = useState(false);
  const [latest, setLatest] = useState<Reading | null>(null);
  const [series, setSeries] = useState<Reading[]>([]);
  const [alerts, setAlerts] = useState<MlAlert[]>([]);
  const [exporting, setExporting] = useState(false);

  const svgRef = useRef<SVGSVGElement | null>(null);
  const eventSourceRef = useRef<EventSource | null>(null);

  const maxPoints = 60;
  const width = 900;
  const height = 260;

  // Get auth token for API calls (now imported from auth.ts)

  // Export FHIR Bundle
  async function exportFhirBundle() {
    setExporting(true);
    try {
      const token = getAuthToken();
      const bundle = await apiGetJson<any>(`/api/fhir/export?limit=100`);
      
      // Download as JSON file
      const blob = new Blob([JSON.stringify(bundle, null, 2)], { type: "application/json" });
      const downloadUrl = window.URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = downloadUrl;
      link.download = `fhir-bundle-${new Date().toISOString().split('T')[0]}.json`;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      window.URL.revokeObjectURL(downloadUrl);
      
      alert("FHIR bundle exported successfully!");
    } catch (error: any) {
      alert(`Export failed: ${error.message}`);
    } finally {
      setExporting(false);
    }
  }

  // Setup SSE connection for real-time updates
  useEffect(() => {
    const apiUrl = import.meta.env.VITE_API_URL || "http://localhost:8080";
    const token = getAuthToken();
    
    // Use EventSource for SSE
    // Note: EventSource doesn't support custom headers, so we'll use query param or cookie
    // For production, use cookie-based auth or implement SSE with fetch + ReadableStream
    const eventSourceUrl = token 
      ? `${apiUrl}/api/stream/vitals?token=${encodeURIComponent(token)}`
      : `${apiUrl}/api/stream/vitals`;
    
    const eventSource = new EventSource(eventSourceUrl);
    
    eventSourceRef.current = eventSource;

    // Listen for vitals events
    eventSource.addEventListener("vitals", (event) => {
      try {
        const data = JSON.parse(event.data);
        const reading: Reading = {
          t: data.timestamp ? data.timestamp * 1000 : Date.now(),
          heartRate: Number(data.heartRate ?? data.hr ?? 0),
          spo2: Number(data.spo2 ?? 0),
          temperature: Number(data.temperature ?? data.temp ?? 0),
        };

        setConnected(true);
        setLatest(reading);
        setSeries((prev) => [...prev.slice(-maxPoints + 1), reading]);
      } catch (e) {
        console.error("Failed to parse vitals event:", e);
      }
    });

    // Listen for alert events
    eventSource.addEventListener("alert", (event) => {
      try {
        const alert: MlAlert = JSON.parse(event.data);
        setAlerts((prev) => [alert, ...prev.slice(0, 4)]); // Keep last 5 alerts
        
        // Show notification
        if (alert.level === "critical") {
          alert(`🚨 CRITICAL ALERT: ${alert.message}`);
        }
      } catch (e) {
        console.error("Failed to parse alert event:", e);
      }
    });

    // Listen for heartbeat
    eventSource.addEventListener("heartbeat", () => {
      setConnected(true);
    });

    // Handle connection errors
    eventSource.onerror = (error) => {
      console.error("SSE connection error:", error);
      setConnected(false);
      // Fallback to polling if SSE fails
      eventSource.close();
      pollOnce();
      const pollInterval = setInterval(pollOnce, 2000);
      return () => clearInterval(pollInterval);
    };

    // Cleanup on unmount
    return () => {
      eventSource.close();
      eventSourceRef.current = null;
    };
  }, []);

  // Fallback polling function (if SSE not available)
  async function pollOnce() {
    try {
      const data = await apiGetJson<BackendLatest>("/api/vitals/latest");

      const reading: Reading = {
        t: Date.now(),
        heartRate: Number(data.heartRate ?? data.hr ?? 0),
        spo2: Number(data.spo2 ?? 0),
        temperature: Number(data.temperature ?? data.temp ?? 0),
      };

      setConnected(true);
      setLatest(reading);
      setSeries((prev) => [...prev.slice(-maxPoints + 1), reading]);
    } catch {
      setConnected(false);
    }
  }

  const hrExtent = useMemo(() => {
    const vals = series.map((d) => d.heartRate).filter((v) => v > 0);
    if (vals.length === 0) return [40, 140] as const;
    const min = Math.min(...vals);
    const max = Math.max(...vals);
    return [Math.max(30, min - 10), Math.min(220, max + 10)] as const;
  }, [series]);

  useEffect(() => {
    if (!svgRef.current) return;

    const svg = d3.select(svgRef.current);
    svg.selectAll("*").remove();

    const margin = { top: 16, right: 16, bottom: 26, left: 44 };
    const innerW = width - margin.left - margin.right;
    const innerH = height - margin.top - margin.bottom;

    const g = svg
      .attr("viewBox", `0 0 ${width} ${height}`)
      .append("g")
      .attr("transform", `translate(${margin.left},${margin.top})`);

    const now = Date.now();
    const fallbackDomain: [Date, Date] = [new Date(now - 120_000), new Date(now)];

    const xDomain = (d3.extent(series, (d) => new Date(d.t)) as [Date, Date]) ?? fallbackDomain;

    const x = d3.scaleTime().domain(xDomain).range([0, innerW]);
    const y = d3.scaleLinear().domain(hrExtent).range([innerH, 0]).nice();

    g.append("g").attr("transform", `translate(0,${innerH})`).call(d3.axisBottom(x).ticks(4));
    g.append("g").call(d3.axisLeft(y).ticks(5));

    const line = d3
      .line<Reading>()
      .defined((d) => d.heartRate > 0)
      .x((d) => x(new Date(d.t)))
      .y((d) => y(d.heartRate));

    g.append("path")
      .datum(series)
      .attr("fill", "none")
      .attr("stroke", "#e74c3c")
      .attr("stroke-width", 2)
      .attr("d", line);

    // latest dot
    const last = series[series.length - 1];
    if (last && last.heartRate > 0) {
      g.append("circle")
        .attr("cx", x(new Date(last.t)))
        .attr("cy", y(last.heartRate))
        .attr("r", 4)
        .attr("fill", "#e74c3c");
    }
  }, [series, hrExtent]);

  function logout() {
    if (eventSourceRef.current) {
      eventSourceRef.current.close();
    }
    clearAuth();
    nav("/auth");
  }

  function getAlertColor(level: string): string {
    switch (level) {
      case "critical":
        return "#e74c3c";
      case "high":
        return "#f39c12";
      case "medium":
        return "#f1c40f";
      default:
        return "#95a5a6";
    }
  }

  return (
    <div className="page">
      <div className="card">
        <div className="header">
          <div>
            <h2>Dashboard</h2>
            <div className={connected ? "status ok" : "status bad"}>
              {connected ? "Connected (SSE)" : "Disconnected"}
            </div>
          </div>

          <div className="header-actions">
            {auth ? <div className="chip">User: {auth.email}</div> : <div className="chip">Guest</div>}
            <button
              className="btn btn-ghost"
              onClick={exportFhirBundle}
              disabled={exporting}
              type="button"
              title="Export FHIR Bundle"
            >
              {exporting ? "Exporting..." : "📥 Export FHIR"}
            </button>
            <button className="btn btn-ghost" onClick={logout} type="button">
              Logout
            </button>
          </div>
        </div>

        {/* ML Alerts Display */}
        {alerts.length > 0 && (
          <div style={{ marginBottom: "1rem" }}>
            <h3 style={{ fontSize: "0.9rem", marginBottom: "0.5rem" }}>Recent Alerts</h3>
            <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
              {alerts.map((alert, idx) => (
                <div
                  key={idx}
                  style={{
                    padding: "0.75rem",
                    borderRadius: "4px",
                    borderLeft: `4px solid ${getAlertColor(alert.level)}`,
                    backgroundColor: `${getAlertColor(alert.level)}20`,
                  }}
                >
                  <div style={{ fontWeight: "bold", color: getAlertColor(alert.level) }}>
                    {alert.level.toUpperCase()}: {alert.message}
                  </div>
                  {alert.details && (
                    <div style={{ fontSize: "0.85rem", marginTop: "0.25rem", color: "#666" }}>
                      {JSON.stringify(alert.details, null, 2)}
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}

        <div className="grid">
          <div className="metric">
            <div className="label">Heart rate</div>
            <div className="value">{latest?.heartRate ?? "--"} bpm</div>
          </div>
          <div className="metric">
            <div className="label">SpO2</div>
            <div className="value">{latest?.spo2 ?? "--"} %</div>
          </div>
          <div className="metric">
            <div className="label">Temperature</div>
            <div className="value">
              {latest ? `${latest.temperature.toFixed(1)} °C` : "--"}
            </div>
          </div>
        </div>

        <h3 className="section-title">Heart rate over time</h3>
        <svg ref={svgRef} width="100%" height="260" />
        <p className="muted">
          Real-time updates via Server-Sent Events (SSE). ML alerts appear above.
        </p>
      </div>
    </div>
  );
}
