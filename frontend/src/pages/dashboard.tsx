import { useEffect, useMemo, useRef, useState } from "react";
import * as d3 from "d3";
import { apiGetJson } from "../lib/api";
import { clearAuth, getAuth } from "../lib/auth";
import { useNavigate } from "react-router-dom";

type BackendLatest = {
  heartRate?: number;
  spo2?: number;
  temperature?: number;

  // optional alternative keys (in case you later standardize differently)
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

export default function Dashboard() {
  const nav = useNavigate();
  const auth = getAuth();

  const [connected, setConnected] = useState(false);
  const [latest, setLatest] = useState<Reading | null>(null);
  const [series, setSeries] = useState<Reading[]>([]);

  const svgRef = useRef<SVGSVGElement | null>(null);

  const maxPoints = 60;
  const width = 900;
  const height = 260;

  async function pollOnce() {
  try {
    const data = await apiGetJson<BackendLatest>("/api/vitals/latest");

    const reading: Reading = {
      t: Date.now(),
      heartRate: Number(data.heartRate ?? data.hr ?? 0),
      spo2: Number(data.spo2 ?? 0),
      temperature: Number(data.temperature ?? data.temp ?? 0)
    };

    setConnected(true);
    setLatest(reading);
    setSeries((prev) => [...prev.slice(-maxPoints + 1), reading]);
  } catch {
    setConnected(false);
  }
}


  useEffect(() => {
    pollOnce();
    const id = window.setInterval(pollOnce, 2000); // like your HTML polling [file:9]
    return () => window.clearInterval(id);
  }, []);

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
    clearAuth();
    nav("/auth");
  }

  return (
    <div className="page">
      <div className="card">
        <div className="header">
          <div>
            <h2>Dashboard</h2>
            <div className={connected ? "status ok" : "status bad"}>
              {connected ? "Connected to backend" : "Disconnected"}
            </div>
          </div>

          <div className="header-actions">
            {auth ? <div className="chip">User: {auth.email}</div> : <div className="chip">Guest</div>}
            <button className="btn btn-ghost" onClick={logout} type="button">
              Logout
            </button>
          </div>
        </div>

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
          Polling <code>/heartrate</code> every 2 seconds (same idea as your previous HTML page). [file:9]
        </p>
      </div>
    </div>
  );
}
