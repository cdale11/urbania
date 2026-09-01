import React from 'react';
import ThreeScene from './ThreeScene';

const App: React.FC = () => {
  return (
    <div style={{ width: '100%', height: '100vh', margin: 0, padding: 0, display: 'flex', flexDirection: 'column' }}>
      <h1 style={{ textAlign: 'center', margin: '0.5rem' }}>Urbania</h1>
      <div style={{ flexGrow: 1 }}>
        <ThreeScene />
      </div>
    </div>
  );
};

export default App;
