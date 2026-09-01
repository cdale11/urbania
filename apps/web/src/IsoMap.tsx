import React, { useEffect, useRef, useState } from 'react';

type WorldPos = { x: number; y: number };
type RoadNode = { id: number; pos: WorldPos; junction: string };
type RoadEdge = { id: number; start: number; end: number; lanes: number; speed_limit: number; width: number; grade: number };
type RoadGraph = { nodes: RoadNode[]; edges: RoadEdge[] };

const TILE_W = 64;
const TILE_H = 32;

function worldToScreen(wx: number, wy: number, offsetX: number, offsetY: number, scale: number) {
  const sx = (wx - wy) * (TILE_W / 2) * scale + offsetX;
  const sy = (wx + wy) * (TILE_H / 2) * scale + offsetY;
  return { x: sx, y: sy };
}
function screenToWorld(sx: number, sy: number, offsetX: number, offsetY: number, scale: number): WorldPos {
  const dx = (sx - offsetX) / scale;
  const dy = (sy - offsetY) / scale;
  const wx = (dx / (TILE_W / 2) + dy / (TILE_H / 2)) / 2;
  const wy = (dy / (TILE_H / 2) - dx / (TILE_W / 2)) / 2;
  return { x: Math.round(wx), y: Math.round(wy) };
}

const IsoMap: React.FC<{ cityId: number }> = ({ cityId }) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [graph, setGraph] = useState<RoadGraph>({ nodes: [], edges: [] });
  const [status, setStatus] = useState<string>('loading');
  const [dragStart, setDragStart] = useState<WorldPos | null>(null);
  const [preview, setPreview] = useState<WorldPos | null>(null);
  const [pan, setPan] = useState({ x: 400, y: 150 });
  const scale = 1;

  // Fetch initial roads + WS
  useEffect(() => {
    let ws: WebSocket | null = null;
    let cancelled = false;
    const load = async () => {
      try {
        const res = await fetch(`/cities/${cityId}/roads`);
        if (res.ok) {
          const dto = await res.json();
          if (!cancelled) setGraph(dto);
        }
        // Also try WS snapshot
        const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
        ws = new WebSocket(`${proto}//${location.host}/cities/${cityId}/ws`);
        ws.onmessage = (ev) => {
          try {
            const msg = JSON.parse(ev.data);
            if (msg.kind === 'snapshot' || msg.Snapshot) {
              // Axum serializes ServerMessage::Snapshot -> {kind:"snapshot", ...} but our Rust uses tag kind snake_case
              const snap = msg.Snapshot ?? msg;
              // Handle both shapes
              const g = msg.road_graph ?? snap.road_graph ?? snap?.Snapshot?.road_graph;
              if (g) setGraph(g);
              else if (snap?.road_graph) setGraph(snap.road_graph);
            }
            // Handle tagged enum: {kind:"snapshot", ...}
            if (msg.kind === 'snapshot' && msg.road_graph) setGraph(msg.road_graph);
            if (msg.kind === 'delta' && msg.changed_roads) setGraph(msg.changed_roads);
            if (msg.kind === 'delta' && msg.Delta?.changed_roads) setGraph(msg.Delta.changed_roads);
            // Fallback: direct Graph
            if (msg.nodes && msg.edges) setGraph(msg);
          } catch {}
        };
        ws.onopen = () => setStatus(`connected city ${cityId}`);
        ws.onerror = () => setStatus('ws error');
      } catch (e) {
        setStatus('failed to load');
      }
    };
    load();
    return () => { cancelled = true; ws?.close(); };
  }, [cityId]);

  // Draw
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const draw = () => {
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      // Grid - draw diamonds for 20x20 range
      ctx.strokeStyle = '#e0e0e0';
      ctx.lineWidth = 1;
      for (let x = -10; x <= 10; x++) {
        for (let y = -10; y <= 10; y++) {
          const p = worldToScreen(x, y, pan.x, pan.y, scale);
          // Diamond tile outline
          ctx.beginPath();
          ctx.moveTo(p.x, p.y - TILE_H/2);
          ctx.lineTo(p.x + TILE_W/2, p.y);
          ctx.lineTo(p.x, p.y + TILE_H/2);
          ctx.lineTo(p.x - TILE_W/2, p.y);
          ctx.closePath();
          ctx.stroke();
        }
      }
      // Roads
      const nodeMap = new Map<number, WorldPos>();
      graph.nodes.forEach(n => nodeMap.set(n.id, n.pos));
      ctx.strokeStyle = '#333';
      ctx.lineWidth = 4;
      ctx.lineCap = 'round';
      graph.edges.forEach(e => {
        const a = nodeMap.get(e.start);
        const b = nodeMap.get(e.end);
        if (!a || !b) return;
        const pa = worldToScreen(a.x, a.y, pan.x, pan.y, scale);
        const pb = worldToScreen(b.x, b.y, pan.x, pan.y, scale);
        ctx.beginPath();
        ctx.moveTo(pa.x, pa.y);
        ctx.lineTo(pb.x, pb.y);
        ctx.stroke();
        // lane markers
        ctx.strokeStyle = '#fff';
        ctx.lineWidth = 1;
        ctx.setLineDash([6, 6]);
        ctx.beginPath();
        ctx.moveTo(pa.x, pa.y);
        ctx.lineTo(pb.x, pb.y);
        ctx.stroke();
        ctx.setLineDash([]);
        ctx.strokeStyle = '#333';
        ctx.lineWidth = 4;
      });
      // Nodes
      graph.nodes.forEach(n => {
        const p = worldToScreen(n.pos.x, n.pos.y, pan.x, pan.y, scale);
        ctx.fillStyle = n.junction === 'Intersection' ? '#d00' : '#555';
        ctx.beginPath();
        ctx.arc(p.x, p.y, 5, 0, Math.PI * 2);
        ctx.fill();
        ctx.strokeStyle = '#fff';
        ctx.lineWidth = 1;
        ctx.stroke();
      });
      // Preview
      if (dragStart && preview) {
        const pa = worldToScreen(dragStart.x, dragStart.y, pan.x, pan.y, scale);
        const pb = worldToScreen(preview.x, preview.y, pan.x, pan.y, scale);
        ctx.strokeStyle = '#0a84ff';
        ctx.lineWidth = 3;
        ctx.setLineDash([8, 4]);
        ctx.beginPath();
        ctx.moveTo(pa.x, pa.y);
        ctx.lineTo(pb.x, pb.y);
        ctx.stroke();
        ctx.setLineDash([]);
      }
    };
    draw();
  }, [graph, pan, dragStart, preview]);

  const canvasPos = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const rect = (e.target as HTMLCanvasElement).getBoundingClientRect();
    const x = (e.clientX - rect.left) * (canvasRef.current!.width / rect.width);
    const y = (e.clientY - rect.top) * (canvasRef.current!.height / rect.height);
    return { x, y };
  };

  const handleDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const { x, y } = canvasPos(e);
    const w = screenToWorld(x, y, pan.x, pan.y, scale);
    setDragStart(w);
    setPreview(w);
  };
  const handleMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!dragStart) return;
    const { x, y } = canvasPos(e);
    const w = screenToWorld(x, y, pan.x, pan.y, scale);
    setPreview(w);
  };
  const handleUp = async (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!dragStart) return;
    const { x, y } = canvasPos(e);
    const end = screenToWorld(x, y, pan.x, pan.y, scale);
    setPreview(null);
    setDragStart(null);
    if (end.x === dragStart.x && end.y === dragStart.y) return;
    try {
      const res = await fetch(`/cities/${cityId}/roads`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ from: dragStart, to: end, lanes: 2 }),
      });
      if (res.ok) {
        const data = await res.json();
        if (data.road_graph) setGraph(data.road_graph);
        else {
          const r = await fetch(`/cities/${cityId}/roads`);
          if (r.ok) setGraph(await r.json());
        }
        setStatus(`road ${dragStart.x},${dragStart.y} -> ${end.x},${end.y} built`);
      } else {
        const txt = await res.text();
        setStatus(`build failed: ${txt}`);
      }
    } catch (err) {
      setStatus(`error: ${String(err)}`);
    }
  };

  // Simple pan with right-drag or shift
  const handleWheel = (e: React.WheelEvent) => {
    // Pan with wheel
    if (e.shiftKey) setPan(p => ({ x: p.x - e.deltaX, y: p.y - e.deltaY }));
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div style={{ padding: '4px 8px', background: '#fafafa', borderBottom: '1px solid #ddd', fontSize: 12, display: 'flex', gap: 12, alignItems: 'center' }}>
        <span>City {cityId}</span>
        <span style={{ color: '#666' }}>{status}</span>
        <span style={{ color: '#888' }}>{graph.nodes.length} nodes / {graph.edges.length} edges</span>
        <span style={{ marginLeft: 'auto', color: '#0a84ff' }}>Click-drag to build road (2 lanes, auto-snap)</span>
      </div>
      <canvas
        ref={canvasRef}
        width={800}
        height={600}
        style={{ width: '100%', height: '100%', cursor: 'crosshair', background: '#f5f7f0' }}
        onMouseDown={handleDown}
        onMouseMove={handleMove}
        onMouseUp={handleUp}
        onWheel={handleWheel}
      />
    </div>
  );
};

export default IsoMap;
