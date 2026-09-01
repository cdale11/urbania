import React, { useEffect, useRef, useState } from 'react';

type WorldPos = { x: number; y: number };
type RoadNode = { id: number; pos: WorldPos; junction: string };
type RoadEdge = { id: number; start: number; end: number; lanes: number; speed_limit: number; width: number; grade: number };
type RoadGraph = { nodes: RoadNode[]; edges: RoadEdge[] };
type ChunkDto = { cx: number; cy: number; data: any };
type ZoneType = 'ResidentialLow'|'ResidentialMedium'|'ResidentialHigh'|'Commercial'|'Office'|'Industrial'|'MixedUse'|'Park';
type ZoneDto = { id: number; city_id: number; x1: number; y1: number; x2: number; y2: number; zone_type: ZoneType; created_at: string };
type ParcelDto = { id: number; zone_id: number; x: number; y: number; w: number; h: number };

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
  if (h < 0.30) return '#3a6ea5';
  if (h < 0.35) return '#d2b48c';
  if (h < 0.50) return '#6a9a3a';
  if (h < 0.65) return '#4a7a2a';
  if (h < 0.80) return '#8a8a8a';
  return '#e8e8e8';
}
const zoneColors: Record<ZoneType, string> = {
  ResidentialLow: 'rgba(127,176,105,0.45)',
  ResidentialMedium: 'rgba(74,124,46,0.45)',
  ResidentialHigh: 'rgba(45,80,22,0.45)',
  Commercial: 'rgba(74,144,226,0.45)',
  Office: 'rgba(106,90,205,0.45)',
  Industrial: 'rgba(139,69,19,0.45)',
  MixedUse: 'rgba(218,165,32,0.45)',
  Park: 'rgba(168,213,162,0.45)',
};

const IsoMap: React.FC<{ cityId: number }> = ({ cityId }) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [graph, setGraph] = useState<RoadGraph>({ nodes: [], edges: [] });
  const [chunks, setChunks] = useState<Map<string, ChunkDto>>(new Map());
  const [zones, setZones] = useState<ZoneDto[]>([]);
  const [parcels, setParcels] = useState<ParcelDto[]>([]);
  const [status, setStatus] = useState<string>('loading');
  const [tool, setTool] = useState<'road'|'zone'>('road');
  const [zoneType, setZoneType] = useState<ZoneType>('ResidentialLow');
  const [dragStart, setDragStart] = useState<WorldPos | null>(null);
  const [preview, setPreview] = useState<WorldPos | null>(null);
  const [zoneDraft, setZoneDraft] = useState<{ x1: number; y1: number; x2: number; y2: number } | null>(null);
  const [pan, setPan] = useState({ x: 400, y: 300 });
  const scale = 1;

  // Fetch roads + zones/parcels + WS
  useEffect(() => {
    let ws: WebSocket | null = null;
    let cancelled = false;
    const load = async () => {
      try {
        const [rRoads, rZones, rParcels] = await Promise.all([
          fetch(`/cities/${cityId}/roads`),
          fetch(`/cities/${cityId}/zones`),
          fetch(`/cities/${cityId}/parcels`),
        ]);
        if (!cancelled) {
          if (rRoads.ok) setGraph(await rRoads.json());
          if (rZones.ok) setZones(await rZones.json());
          if (rParcels.ok) setParcels(await rParcels.json());
        }
        const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
        ws = new WebSocket(`${proto}//${location.host}/cities/${cityId}/ws`);
        ws.onmessage = (ev) => {
          try {
            const msg = JSON.parse(ev.data);
            if (msg.kind === 'snapshot' && msg.road_graph) setGraph(msg.road_graph);
            if (msg.kind === 'snapshot' && msg.zones) setZones(msg.zones);
            if (msg.kind === 'snapshot' && msg.parcels) setParcels(msg.parcels);
            if (msg.kind === 'delta' && msg.changed_roads) setGraph(msg.changed_roads);
            if (msg.kind === 'delta' && msg.changed_zones) setZones(msg.changed_zones);
            if (msg.kind === 'delta' && msg.changed_parcels) setParcels(msg.changed_parcels);
            if (msg.Snapshot?.road_graph) setGraph(msg.Snapshot.road_graph);
            if (msg.Snapshot?.zones) setZones(msg.Snapshot.zones);
            if (msg.Delta?.changed_roads) setGraph(msg.Delta.changed_roads);
            if (msg.Delta?.changed_zones) setZones(msg.Delta.changed_zones);
            if (msg.nodes && msg.edges) setGraph(msg);
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

  // Fetch terrain chunks 5x5
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
    };
    setChunks(new Map());
    fetchChunks();
    return () => { cancelled = true; };
  }, [cityId]);

  // Draw
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const draw = () => {
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      // Terrain
      chunks.forEach(chunk => {
        const heights: number[] | undefined = chunk.data?.heights;
        const vegetation: number[] | undefined = chunk.data?.vegetation;
        const size: number = chunk.data?.size ?? CHUNK_SIZE;
        if (!heights) return;
        for (let ly = 0; ly < size; ly++) {
          for (let lx = 0; lx < size; lx++) {
            const idx = ly * size + lx;
            const h = heights[idx];
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
            ctx.strokeStyle = 'rgba(0,0,0,0.06)';
            ctx.lineWidth = 1;
            ctx.stroke();
            if (vegetation && vegetation[idx] > 0) {
              ctx.fillStyle = 'rgba(34,100,34,0.9)';
              ctx.beginPath();
              ctx.arc(p.x, p.y, 2, 0, Math.PI * 2);
              ctx.fill();
            }
          }
        }
      });
      // Zones (filled rects as isometric polygons)
      zones.forEach(z => {
        const x1 = Math.min(z.x1, z.x2), x2 = Math.max(z.x1, z.x2);
        const y1 = Math.min(z.y1, z.y2), y2 = Math.max(z.y1, z.y2);
        const corners = [
          worldToScreen(x1, y1, pan.x, pan.y, scale),
          worldToScreen(x2, y1, pan.x, pan.y, scale),
          worldToScreen(x2, y2, pan.x, pan.y, scale),
          worldToScreen(x1, y2, pan.x, pan.y, scale),
        ];
        ctx.fillStyle = zoneColors[z.zone_type] ?? 'rgba(0,0,0,0.2)';
        ctx.beginPath();
        ctx.moveTo(corners[0].x, corners[0].y);
        corners.slice(1).forEach(c => ctx.lineTo(c.x, c.y));
        ctx.closePath();
        ctx.fill();
        ctx.strokeStyle = zoneColors[z.zone_type]?.replace('0.45', '0.9') ?? '#000';
        ctx.lineWidth = 2;
        ctx.stroke();
      });
      // Parcels (thin dashed inside zones)
      ctx.strokeStyle = 'rgba(0,0,0,0.35)';
      ctx.lineWidth = 1;
      ctx.setLineDash([4, 4]);
      parcels.forEach(p => {
        const c1 = worldToScreen(p.x, p.y, pan.x, pan.y, scale);
        const c2 = worldToScreen(p.x + p.w, p.y, pan.x, pan.y, scale);
        const c3 = worldToScreen(p.x + p.w, p.y + p.h, pan.x, pan.y, scale);
        const c4 = worldToScreen(p.x, p.y + p.h, pan.x, pan.y, scale);
        ctx.beginPath();
        ctx.moveTo(c1.x, c1.y);
        ctx.lineTo(c2.x, c2.y);
        ctx.lineTo(c3.x, c3.y);
        ctx.lineTo(c4.x, c4.y);
        ctx.closePath();
        ctx.stroke();
      });
      ctx.setLineDash([]);
      // Faint grid
      ctx.strokeStyle = 'rgba(0,0,0,0.10)';
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
      // Zone draft preview
      if (zoneDraft) {
        const x1 = Math.min(zoneDraft.x1, zoneDraft.x2), x2 = Math.max(zoneDraft.x1, zoneDraft.x2);
        const y1 = Math.min(zoneDraft.y1, zoneDraft.y2), y2 = Math.max(zoneDraft.y1, zoneDraft.y2);
        const corners = [
          worldToScreen(x1, y1, pan.x, pan.y, scale),
          worldToScreen(x2, y1, pan.x, pan.y, scale),
          worldToScreen(x2, y2, pan.x, pan.y, scale),
          worldToScreen(x1, y2, pan.x, pan.y, scale),
        ];
        ctx.fillStyle = zoneColors[zoneType]?.replace('0.45', '0.25') ?? 'rgba(10,132,255,0.2)';
        ctx.beginPath();
        ctx.moveTo(corners[0].x, corners[0].y);
        corners.slice(1).forEach(c => ctx.lineTo(c.x, c.y));
        ctx.closePath();
        ctx.fill();
        ctx.strokeStyle = '#0a84ff';
        ctx.lineWidth = 2;
        ctx.setLineDash([8, 4]);
        ctx.stroke();
        ctx.setLineDash([]);
      }
      // Road preview
      if (dragStart && preview && tool === 'road') {
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
  }, [graph, chunks, zones, parcels, pan, dragStart, preview, zoneDraft, zoneType, tool]);

  const canvasPos = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const rect = (e.target as HTMLCanvasElement).getBoundingClientRect();
    const x = (e.clientX - rect.left) * (canvasRef.current!.width / rect.width);
    const y = (e.clientY - rect.top) * (canvasRef.current!.height / rect.height);
    return { x, y };
  };

  const handleDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const { x, y } = canvasPos(e);
    const w = screenToWorld(x, y, pan.x, pan.y, scale);
    if (tool === 'zone') {
      setZoneDraft({ x1: w.x, y1: w.y, x2: w.x, y2: w.y });
    } else {
      setDragStart(w);
      setPreview(w);
    }
  };
  const handleMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const { x, y } = canvasPos(e);
    const w = screenToWorld(x, y, pan.x, pan.y, scale);
    if (tool === 'zone' && zoneDraft) {
      setZoneDraft({ ...zoneDraft, x2: w.x, y2: w.y });
    } else if (tool === 'road' && dragStart) {
      setPreview(w);
    }
  };
  const handleUp = async (e: React.MouseEvent<HTMLCanvasElement>) => {
    const { x, y } = canvasPos(e);
    const end = screenToWorld(x, y, pan.x, pan.y, scale);
    if (tool === 'zone' && zoneDraft) {
      const draft = { x1: zoneDraft.x1, y1: zoneDraft.y1, x2: end.x, y2: end.y };
      setZoneDraft(null);
      if (draft.x1 === draft.x2 || draft.y1 === draft.y2) return;
      const x1 = Math.min(draft.x1, draft.x2), y1 = Math.min(draft.y1, draft.y2);
      const x2 = Math.max(draft.x1, draft.x2), y2 = Math.max(draft.y1, draft.y2);
      if (Math.abs(x2 - x1) > 30 || Math.abs(y2 - y1) > 30) { setStatus('zone too large (max 30)'); return; }
      try {
        const res = await fetch(`/cities/${cityId}/zones`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ x1, y1, x2, y2, zone_type: zoneType }),
        });
        if (res.ok) {
          const z = await res.json();
          setZones(zs => [...zs, z]);
          // Refresh parcels
          const rp = await fetch(`/cities/${cityId}/parcels`);
          if (rp.ok) setParcels(await rp.json());
          setStatus(`zone ${zoneType} ${x1},${y1}→${x2},${y2} created`);
        } else {
          const txt = await res.text();
          setStatus(`zone failed: ${txt}`);
        }
      } catch (err) {
        setStatus(`error: ${String(err)}`);
      }
      return;
    }
    if (tool === 'road') {
      if (!dragStart) return;
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
    }
  };

  const handleWheel = (e: React.WheelEvent) => {
    if (e.shiftKey) setPan(p => ({ x: p.x - e.deltaX, y: p.y - e.deltaY }));
  };
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
      <div style={{ padding: '4px 8px', background: '#fafafa', borderBottom: '1px solid #ddd', fontSize: 12, display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap' }}>
        <span>City {cityId}</span>
        <span style={{ color: '#666' }}>{status}</span>
        <span style={{ color: '#888' }}>{graph.nodes.length} nodes / {graph.edges.length} edges | {zones.length} zones / {parcels.length} parcels | {chunks.size} chunks</span>
        <div style={{ marginLeft: 'auto', display: 'flex', gap: 6, alignItems: 'center' }}>
          <button onClick={() => setTool('road')} style={{ padding: '4px 8px', background: tool === 'road' ? '#0a84ff' : '#eee', color: tool === 'road' ? '#fff' : '#000', border: '1px solid #ccc' }}>Road</button>
          <button onClick={() => setTool('zone')} style={{ padding: '4px 8px', background: tool === 'zone' ? '#0a84ff' : '#eee', color: tool === 'zone' ? '#fff' : '#000', border: '1px solid #ccc' }}>Zone</button>
          <select value={zoneType} onChange={e => setZoneType(e.target.value as ZoneType)} style={{ padding: 4 }} disabled={tool !== 'zone'}>
            <option value="ResidentialLow">Res Low</option>
            <option value="ResidentialMedium">Res Med</option>
            <option value="ResidentialHigh">Res High</option>
            <option value="Commercial">Commercial</option>
            <option value="Office">Office</option>
            <option value="Industrial">Industrial</option>
            <option value="MixedUse">Mixed</option>
            <option value="Park">Park</option>
          </select>
          <span style={{ color: '#0a84ff', fontSize: 11 }}>{tool === 'road' ? 'Drag to build road' : `Drag to paint ${zoneType}`}</span>
        </div>
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
      <div style={{ padding: '2px 8px', fontSize: 10, color: '#666', background: '#f5f7f0', display: 'flex', gap: 8, flexWrap: 'wrap' }}>
        <span style={{ background: '#3a6ea5', color: '#fff', padding: '0 4px' }}>water</span>
        <span style={{ background: '#d2b48c', padding: '0 4px' }}>sand</span>
        <span style={{ background: '#6a9a3a', color: '#fff', padding: '0 4px' }}>grass</span>
        <span style={{ background: '#8a8a8a', color: '#fff', padding: '0 4px' }}>rock</span>
        <span style={{ background: '#e8e8e8', padding: '0 4px' }}>snow</span>
        <span style={{ background: zoneColors.ResidentialLow, padding: '0 4px' }}>Res Low</span>
        <span style={{ background: zoneColors.Commercial, padding: '0 4px' }}>Com</span>
        <span style={{ background: zoneColors.Industrial, padding: '0 4px' }}>Ind</span>
        <span>• green dot = vegetation</span>
      </div>
    </div>
  );
};

export default IsoMap;
