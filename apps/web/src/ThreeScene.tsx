import React, { useEffect, useRef } from 'react';
import * as THREE from 'three';

// Simple utility to detect WebGPU support; falls back to WebGL2.
function getRenderer(container: HTMLElement): THREE.Renderer {
  // WebGPU detection (experimental). If unavailable, use WebGLRenderer.
  // This placeholder merely attempts to create a WebGL2 renderer.
  const canvas = document.createElement('canvas');
  const context = canvas.getContext('webgl2') as WebGL2RenderingContext | null;
  if (context) {
    const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
    renderer.setPixelRatio(window.devicePixelRatio);
    renderer.setSize(container.clientWidth, container.clientHeight);
    return renderer;
  }
  // As a fallback, use the basic WebGLRenderer (will also work if webgl2 is not supported).
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
  renderer.setPixelRatio(window.devicePixelRatio);
  renderer.setSize(container.clientWidth, container.clientHeight);
  return renderer;
}

const ThreeScene: React.FC = () => {
  const mountRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!mountRef.current) return;

    const container = mountRef.current;
    const renderer = getRenderer(container);
    // Append the renderer's canvas element to the container so it becomes visible.
    container.appendChild(renderer.domElement);
    const scene = new THREE.Scene();

    // Camera – basic perspective camera positioned at (0, 5, 10) looking at origin.
    const camera = new THREE.PerspectiveCamera(
      60,
      container.clientWidth / container.clientHeight,
      0.1,
      1000,
    );
    camera.position.set(0, 5, 10);
    camera.lookAt(0, 0, 0);

    // Simple ambient light and a grid helper to visualise the floor.
    const ambient = new THREE.AmbientLight(0xffffff, 0.8);
    scene.add(ambient);
    const grid = new THREE.GridHelper(100, 100);
    scene.add(grid);

    // Resize handling.
    const handleResize = () => {
      const width = container.clientWidth;
      const height = container.clientHeight;
      renderer.setSize(width, height);
      camera.aspect = width / height;
      camera.updateProjectionMatrix();
    };
    window.addEventListener('resize', handleResize);

    // Animation loop.
    const animate = () => {
      requestAnimationFrame(animate);
      renderer.render(scene, camera);
    };
    animate();

    // Cleanup on unmount.
    return () => {
      window.removeEventListener('resize', handleResize);
      renderer.dispose();
    };
  }, []);

  return <div ref={mountRef} style={{ width: '100%', height: '100vh' }} />;
};

export default ThreeScene;
