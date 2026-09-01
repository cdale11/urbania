import React, { useEffect, useState } from 'react';
import IsoMap from './IsoMap';
import ThreeScene from './ThreeScene';

type CityMeta = { id: number; name: string; seed: number; tick: number; created_at: string };

const App: React.FC = () => {
  const [cities, setCities] = useState<CityMeta[]>([]);
  const [selected, setSelected] = useState<number | null>(null);
  const [newName, setNewName] = useState('');
  const [view, setView] = useState<'iso' | '3d'>('iso');

  const loadCities = async () => {
    const res = await fetch('/cities');
    if (res.ok) {
      const data = await res.json();
      setCities(data);
      if (data.length && selected === null) setSelected(data[0].id);
    }
  };
  useEffect(() => { loadCities(); }, []);

  const createCity = async () => {
    const name = newName.trim() || `City ${cities.length + 1}`;
    const res = await fetch('/cities', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ name }) });
    if (res.ok) {
      const { city } = await res.json();
      setCities(c => [...c, city]);
      setSelected(city.id);
      setNewName('');
    }
  };

  return (
    <div style={{ width: '100%', height: '100vh', margin: 0, padding: 0, display: 'flex', flexDirection: 'column', fontFamily: 'system-ui, sans-serif' }}>
      <header style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '8px 12px', borderBottom: '1px solid #ddd', background: '#fff' }}>
        <h1 style={{ margin: 0, fontSize: 18 }}>Urbania</h1>
        <div style={{ display: 'flex', gap: 6, alignItems: 'center', marginLeft: 12 }}>
          <select value={selected ?? ''} onChange={e => setSelected(Number(e.target.value))} style={{ padding: 4 }}>
            {cities.map(c => <option key={c.id} value={c.id}>{c.name} (#{c.id})</option>)}
          </select>
          <input placeholder="New city name" value={newName} onChange={e => setNewName(e.target.value)} style={{ padding: 4 }} />
          <button onClick={createCity} style={{ padding: '4px 8px' }}>Create</button>
          <button onClick={loadCities} style={{ padding: '4px 8px' }}>Refresh</button>
        </div>
        <div style={{ marginLeft: 'auto', display: 'flex', gap: 8 }}>
          <button onClick={() => setView('iso')} style={{ padding: '4px 10px', background: view === 'iso' ? '#0a84ff' : '#eee', color: view === 'iso' ? '#fff' : '#000', border: '1px solid #ccc' }}>2.5D Isometric (Roads)</button>
          <button onClick={() => setView('3d')} style={{ padding: '4px 10px', background: view === '3d' ? '#0a84ff' : '#eee', color: view === '3d' ? '#fff' : '#000', border: '1px solid #ccc' }}>3D Terrain</button>
        </div>
      </header>
      {/* Ribbon per spec 31-32 */}
      <div style={{ display: 'flex', gap: 8, padding: '6px 12px', background: '#f0f0f0', borderBottom: '1px solid #ddd', fontSize: 13 }}>
        <span style={{ fontWeight: 600 }}>Build</span> <span>Road</span> <span style={{ color: '#999' }}>|</span> <span>Zone</span> <span>Services</span> <span>Transit</span> <span>Utilities</span> <span>Policies</span>
        <span style={{ marginLeft: 'auto', color: '#666' }}>{selected ? `City #${selected}` : 'No city selected'}</span>
      </div>
      <div style={{ flexGrow: 1, minHeight: 0 }}>
        {selected ? (
          view === 'iso' ? <IsoMap cityId={selected} /> : <ThreeScene />
        ) : (
          <div style={{ padding: 24, color: '#666' }}>Create or select a city to start building roads. Drag on the isometric grid to place roads (auto-snap, 2 lanes, validation per spec 54).</div>
        )}
      </div>
    </div>
  );
};

export default App;
