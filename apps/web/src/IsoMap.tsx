import React, { useEffect, useRef, useState } from 'react';

type WorldPos = { x: number; y: number };
type RoadNode = { id: number; pos: WorldPos; junction: string };
type RoadEdge = { id: number; start: number; end: number; lanes: number; speed_limit: number; width: number; grade: number };
type RoadGraph = { nodes: RoadNode[]; edges: RoadEdge[] };
type ChunkDto = { cx: number; cy: number; data: any };

const TILE_W = 64;
const TILE_H = 32;
const CHUNK_SIZE = 16;

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
function heightToColor(h: number): string {
  if (h < 0.30) return '#3a6ea5'; // water
  if (h < 0.35) return '#d2b48c'; // sand
  if (h < 0.50) return '#6a9a3a'; // grass low
  if (h < 0.65) return '#4a7a2a'; // grass high
  if (h < 0.80) return '#8a8a8a'; // rock
  return '#e8e8e8'; // snow
}

const IsoMap: React.FC<{ cityId: number }> = ({ cityId }) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [graph, setGraph] = useState<RoadGraph>({ nodes: [], edges: [] });
  const [chunks, setChunks] = useState<Map<string, ChunkDto>>(new Map());
  const [status, setStatus] = useState<string>('loading');
  const [dragStart, setDragStart] = useState<WorldPos | null>(null);
  const [preview, setPreview] = useState<WorldPos | null>(null);
  const [pan, setPan] = useState({ x: 400, y: 300 });
  const scale = 1;

  // Fetch roads + WS
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
        const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
        ws = new WebSocket(`${proto}//${location.host}/cities/${cityId}/ws`);
        ws.onmessage = (ev) => {
          try {
            const msg = JSON.parse(ev.data);
            if (msg.kind === 'snapshot' && msg.road_graph) setGraph(msg.road_graph);
            if (msg.kind === 'delta' && msg.changed_roads) setGraph(msg.changed_roads);
            if (msg.Snapshot?.road_graph) setGraph(msg.Snapshot.road_graph);
            if (msg.Delta?.changed_roads) setGraph(msg.Delta.changed_roads);
            if (msg.nodes && msg.edges) setGraph(msg);
            // Chunk deltas
            if (msg.kind === 'delta' && msg.changed_chunks?.length) {
              setChunks(prev => {
                const m = new Map(prev);
                for (const ch of msg.changed_chunks) m.set(`${ch.cx},${ch.cy}`, ch);
                return m;
              });
            }
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

  // Fetch terrain chunks - 5x5 around origin
  useEffect(() => {
    let cancelled = false;
    const fetchChunks = async () => {
      const promises: Promise<void>[] = [];
      for (let cx = -2; cx <= 2; cx++) {
        for (let cy = -2; cy <= 2; cy++) {
          promises.push(
            fetch(`/cities/${cityId}/chunks/${cx}/${cy}`)
              .then(r => r.ok ? r.json() : null)
              .then(dto => {
                if (dto && !cancelled) {
                  setChunks(prev => {
                    const m = new Map(prev);
                    m.set(`${dto.cx},${dto.cy}`, dto);
                    return m;
                  });
                }
              })
              .catch(() => {})
          );
        }
      }
      await Promise.all(promises);
      if (!cancelled) setStatus(s => s.includes('connected') ? s : `terrain ${chunks.size} chunks`);
    };
    setChunks(new Map());
    fetchChunks();
    return () => { cancelled = true; };
  }, [cityId]);

  // Draw - terrain + grid + roads
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const draw = () => {
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      // Terrain chunks
      chunks.forEach(chunk => {
        const heights: number[] | undefined = chunk.data?.heights;
        const size: number = chunk.data?.size ?? CHUNK_SIZE;
        if (!heights) return;
        for (let ly = 0; ly < size; ly++) {
          for (let lx = 0; lx < size; lx++) {
            const h = heights[ly * size + lx];
            const wx = chunk.cx * size + lx;
            const wy = chunk.cy * size + ly;
            const p = worldToScreen(wx, wy, pan.x, pan.y, scale);
            ctx.fillStyle = heightToColor(h);
            ctx.beginPath();
            ctx.moveTo(p.x, p.y - TILE_H / 2);
            ctx.lineTo(p.x + TILE_W / 2, p.y);
            ctx.lineTo(p.x, p.y + TILE_H / 2);
            ctx.lineTo(p.x - TILE_W / 2, p.y);
            ctx.closePath();
            ctx.fill();
            // subtle stroke for terrain definition
            ctx.strokeStyle = 'rgba(0,0,0,0.08)';
            ctx.lineWidth = 1;
            ctx.stroke();
          }
        }
      });
      // Grid overlay (faint)
      ctx.strokeStyle = 'rgba(0,0,0,0.12)';
      ctx.lineWidth = 1;
      for (let x = -10; x <= 10; x++) {
        for (let y = -10; y <= 10; y++) {
          const p = worldToScreen(x, y, pan.x, pan.y, scale);
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
      ctx.strokeStyle = '#222';
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
        ctx.strokeStyle = '#fff';
        ctx.lineWidth = 1;
        ctx.setLineDash([6, 6]);
        ctx.beginPath();
        ctx.moveTo(pa.x, pa.y);
        ctx.lineTo(pb.x, pb.y);
        ctx.stroke();
        ctx.setLineDash([]);
        ctx.strokeStyle = '#222';
        ctx.lineWidth = 4;
      });
      graph.nodes.forEach(n => {
        const p = worldToScreen(n.pos.x, n.pos.y, pan.x, pan.y, scale);
        ctx.fillStyle = n.junction === 'Intersection' ? '#d00' : '#333';
        ctx.beginPath();
        ctx.arc(p.x, p.y, 5, 0, Math.PI * 2);
        ctx.fill();
        ctx.strokeStyle = '#fff';
        ctx.lineWidth = 1;
        ctx.stroke();
      });
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
  }, [graph, chunks, pan, dragStart, preview]);

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

  const handleWheel = (e: React.WheelEvent) => {
    if (e.shiftKey) setPan(p => ({ x: p.x - e.deltaX, y: p.y - e.deltaY }));
  };

  // Pan with middle mouse
  const handleMouseDownPan = (e: React.MouseEvent) => {
    if (e.button === 1) {
      const start = { x: e.clientX, y: e.clientY, pan: { ...pan } };
      const onMove = (ev: MouseEvent) => setPan({ x: start.pan.x + (ev.clientX - start.x), y: start.pan.y + (ev.clientY - start.y) });
      const onUp = () => { window.removeEventListener('mousemove', onMove); window.removeEventListener('mouseup', onUp); };
      window.addEventListener('mousemove', onMove);
      window.addEventListener('mouseup', onUp);
    }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div style={{ padding: '4px 8px', background: '#fafafa', borderBottom: '1px solid #ddd', fontSize: 12, display: 'flex', gap: 12, alignItems: 'center' }}>
        <span>City {cityId}</span>
        <span style={{ color: '#666' }}>{status}</span>
        <span style={{ color: '#888' }}>{graph.nodes.length} nodes / {graph.edges.length} edges | {chunks.size} chunks</span>
        <span style={{ marginLeft: 'auto', color: '#0a84ff' }}>Click-drag to build road • Shift+wheel/middle-drag to pan</span>
      </div>
      <canvas
        ref={canvasRef}
        width={800}
        height={600}
        style={{ width: '100%', height: '100%', cursor: 'crosshair', background: '#cfe8ff' }}
        onMouseDown={e => { handleMouseDownPan(e); handleDown(e); }}
        onMouseMove={handleMove}
        onMouseUp={handleUp}
        onWheel={handleWheel}
      />
      <div style={{ padding: '2px 8px', fontSize: 10, color: '#666', background: '#f5f7f0', display: 'flex', gap: 8 }}>
        <span style={{ background: '#3a6ea5', color: '#fff', padding: '0 4px' }}>water &lt;0.30</span>
        <span style={{ background: '#d2b48c', padding: '0 4px' }}>sand</span>
        <span style={{ background: '#6a9a3a', color: '#fff', padding: '0 4px' }}>grass</span>
        <span style={{ background: '#8a8a8a', color: '#fff', padding: '0 4px' }}>rock &gt;0.65</span>
        <span style={{ background: '#e8e8e8', padding: '0 4px' }}>snow &gt;0.80</span>
      </div>
    </div>
  );
};

export default IsoMap;
