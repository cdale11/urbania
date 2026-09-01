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
  // As a fallback, use the basic WebGLRenderer.
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
  renderer.setPixelRatio(window.devicePixelRatio);
  renderer.setSize(container.clientWidth, container.clientHeight);
  return renderer;
}

const TERRAIN_SIZE = 64; // vertices per side

const ThreeScene: React.FC = () => {
  const mountRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!mountRef.current) return;

    let animationId: number;
    const container = mountRef.current;
    const renderer = getRenderer(container);
    // Attach canvas to DOM
    container.appendChild(renderer.domElement);
    const scene = new THREE.Scene();

    // Camera
    const camera = new THREE.PerspectiveCamera(
      60,
      container.clientWidth / container.clientHeight,
      0.1,
      1000,
    );
    camera.position.set(0, 20, 30);
    camera.lookAt(0, 0, 0);

    // Ambient + directional light
    const ambient = new THREE.AmbientLight(0xffffff, 0.4);
    scene.add(ambient);
    const dirLight = new THREE.DirectionalLight(0xffffff, 0.6);
    dirLight.position.set(10, 20, 10);
    scene.add(dirLight);

    // Load wasm, generate terrain, and add to scene
    const initTerrain = async () => {
      try {
        // Dynamically import the wasm-bindgen generated module
        const wasm = await import('/pkg/sim_core.js');
        // Initialise the wasm module (calls the generated `__wbindgen_start`)
        await wasm.default();

        const seed = 42; // deterministic seed for now
        const heights = wasm.wasm_generate_height_map(seed, 0, 0, TERRAIN_SIZE);
        // heights is a Float32Array (thanks to wasm-bindgen)

        // Create a plane geometry whose vertices we will displace using the height map
        const geometry = new THREE.PlaneGeometry(
          TERRAIN_SIZE,
          TERRAIN_SIZE,
          TERRAIN_SIZE - 1,
          TERRAIN_SIZE - 1,
        );

        const position = geometry.attributes.position as THREE.BufferAttribute;
        for (let i = 0; i < position.count; i++) {
          // Scale the height to a more visible range
          position.setZ(i, heights[i] * 5);
        }
        position.needsUpdate = true;
        geometry.computeVertexNormals();

        const material = new THREE.MeshStandardMaterial({
          color: 0x5566aa,
          side: THREE.DoubleSide,
          flatShading: false,
        });

        const mesh = new THREE.Mesh(geometry, material);
        mesh.rotation.x = -Math.PI / 2; // make the plane lie on the XZ plane
        scene.add(mesh);
      } catch (e) {
        console.error('Failed to initialize terrain via wasm:', e);
      }
    };
    initTerrain();

    // Resize handling
    const handleResize = () => {
      const width = container.clientWidth;
      const height = container.clientHeight;
      renderer.setSize(width, height);
      camera.aspect = width / height;
      camera.updateProjectionMatrix();
    };
    window.addEventListener('resize', handleResize);

    // Animation loop
    const animate = () => {
      animationId = requestAnimationFrame(animate);
      renderer.render(scene, camera);
    };
    animate();

    // Cleanup on component unmount
    return () => {
      window.removeEventListener('resize', handleResize);
      if (animationId) cancelAnimationFrame(animationId);
      renderer.dispose();
    };
  }, []);

  return <div ref={mountRef} style={{ width: '100%', height: '100vh' }} />;
};

export default ThreeScene;
