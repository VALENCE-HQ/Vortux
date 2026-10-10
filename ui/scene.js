// 3D background: floating cubes textured with community screenshots, plus a starfield.
// During an install the cubes build a random shape as progress grows.
// Needs ui/vendor/three.module.min.js (see scripts/fetch-three.sh).
import * as THREE from "./vendor/three.module.min.js";

const N = 55;
const V3 = (x, y, z) => new THREE.Vector3(x, y, z);

// Each layout: N slots (filled in order), a cube scale, the height the camera looks at.
const LAYOUTS = {
  pyramid() {
    const slots = [];
    for (let layer = 0; layer < 5; layer++) {
      const n = 5 - layer;
      for (let x = 0; x < n; x++)
        for (let z = 0; z < n; z++) slots.push(V3((x - (n - 1) / 2) * 1.08, -2.4 + layer * 1.08, (z - (n - 1) / 2) * 1.08));
    }
    return { slots, scale: 1, y: -0.4 };
  },
  vortex() { // a funnel spiralling up, like the name says
    const slots = Array.from({ length: N }, (_, i) => {
      const t = i / (N - 1), a = t * Math.PI * 8, r = 0.5 + t * 3.2;
      return V3(Math.cos(a) * r, -2.8 + t * 5.4, Math.sin(a) * r);
    });
    return { slots, scale: 0.8, y: 0 };
  },
  v() { // a big V
    const slots = Array.from({ length: N }, (_, i) => {
      const arm = Math.floor(i / 2) % 2 ? 1 : -1, k = Math.floor(i / 4), d = i % 2;
      return V3(arm * (k * 0.34 + 0.2), -3.3 + k * 0.52, (d - 0.5) * 0.6);
    });
    return { slots, scale: 0.55, y: -0.1, swing: true };
  },
  ring() { // three rings, inner first
    const slots = [];
    [[1.6, 10], [2.8, 18], [4.0, 27]].forEach(([r, n]) => {
      for (let j = 0; j < n; j++) {
        const a = (j / n) * Math.PI * 2;
        slots.push(V3(Math.cos(a) * r, Math.sin(a * 3) * 0.25 - 0.5, Math.sin(a) * r));
      }
    });
    return { slots, scale: 0.85, y: -0.5 };
  },
};

export async function startScene(canvas, textureUrls) {
  const renderer = new THREE.WebGLRenderer({ canvas, alpha: true, antialias: true });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(50, 1, 0.1, 100);
  scene.add(new THREE.AmbientLight(0xffffff, 2.2));
  const sun = new THREE.DirectionalLight(0xcdb8ff, 2.4);
  sun.position.set(4, 8, 6);
  scene.add(sun);

  // Starfield
  const pos = new Float32Array(450 * 3);
  for (let i = 0; i < pos.length; i += 3) {
    const r = 14 + Math.random() * 16, th = Math.random() * Math.PI * 2, ph = Math.acos(2 * Math.random() - 1);
    pos[i] = r * Math.sin(ph) * Math.cos(th);
    pos[i + 1] = r * Math.cos(ph);
    pos[i + 2] = r * Math.sin(ph) * Math.sin(th);
  }
  const starGeo = new THREE.BufferGeometry();
  starGeo.setAttribute("position", new THREE.BufferAttribute(pos, 3));
  const stars = new THREE.Points(starGeo, new THREE.PointsMaterial({ size: 0.07, color: 0xdccfff, transparent: true, opacity: 0.8 }));
  scene.add(stars);

  const loader = new THREE.TextureLoader();
  const textures = textureUrls.map((u) => {
    const t = loader.load(u);
    t.colorSpace = THREE.SRGBColorSpace;
    return t;
  });
  const geo = new THREE.BoxGeometry(1, 1, 1);
  const rnd = (a) => (Math.random() - 0.5) * a;
  const cubes = Array.from({ length: N }, () => {
    const map = textures.length ? textures[Math.floor(Math.random() * textures.length)] : null;
    const mesh = new THREE.Mesh(geo, new THREE.MeshLambertMaterial({ map, color: map ? 0xffffff : 0x8b5cf6 }));
    scene.add(mesh);
    return { mesh, slot: V3(0, 0, 0), mix: 0, phase: Math.random() * 6, spin: [rnd(0.8), rnd(0.8), rnd(0.8)],
      r: [rnd(6), rnd(6), rnd(6)], home: V3(rnd(26), rnd(14), -3 - Math.random() * 12) };
  });

  let layout, mode = "float", progress = 0, pulse = 0, mx = 0, my = 0, last = performance.now(), enabled = true, calm = false;
  const look = V3(0, 0, 0);
  function setLayout(name) {
    layout = (LAYOUTS[name] || LAYOUTS.pyramid)();
    cubes.forEach((c, i) => { c.slot = layout.slots[i]; });
  }
  setLayout("pyramid");

  window.addEventListener("pointermove", (e) => {
    mx = e.clientX / window.innerWidth - 0.5;
    my = e.clientY / window.innerHeight - 0.5;
  });
  function resize() {
    const w = canvas.clientWidth, h = canvas.clientHeight;
    if (!w || !h) return;
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
  }
  new ResizeObserver(resize).observe(canvas);
  resize();

  function frame(now) {
    if (!enabled) return;
    const dt = Math.min(0.05, (now - last) / 1000);
    last = now;
    pulse = Math.max(0, pulse - dt * 1.2);
    const bump = 1 + Math.sin(pulse * Math.PI) * 0.3;
    const live = calm ? 0 : 1; // calm mode: no floating or spinning
    const built = mode === "build" ? Math.round(progress * N) : 0;
    cubes.forEach((c, i) => {
      c.mix += ((i < built ? 1 : 0) - c.mix) * Math.min(1, dt * 2.5);
      const e = c.mix * c.mix * (3 - 2 * c.mix);
      c.mesh.position.lerpVectors(c.home, c.slot, e);
      c.mesh.position.y += Math.sin(now / 1000 + c.phase) * 0.35 * live * (1 - e);
      for (let k = 0; k < 3; k++) c.r[k] += c.spin[k] * dt * live;
      c.mesh.rotation.set(c.r[0] * (1 - e), c.r[1] * (1 - e), c.r[2] * (1 - e));
      c.mesh.scale.setScalar((1 + (layout.scale - 1) * e) * bump);
    });
    stars.rotation.y += dt * 0.012;
    const a = mode !== "build" ? 0 : layout.swing ? Math.sin(now / 3000) * 0.6 : now / 5000;
    camera.position.x += (Math.sin(a) * 11 + mx * 3 * live - camera.position.x) * dt * 2;
    camera.position.y += (2.2 - my * 2 * live - camera.position.y) * dt * 2;
    camera.position.z += (Math.cos(a) * 11 + (mode === "build" ? 0 : 3) - camera.position.z) * dt * 2;
    look.y += ((mode === "build" ? layout.y : 0) - look.y) * dt * 2;
    camera.lookAt(look);
    renderer.render(scene, camera);
    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);

  return {
    setMode: (m) => { mode = m; },
    setCalm: (v) => { calm = v; },
    setEnabled: (v) => {
      if (v && !enabled) {
        enabled = true;
        last = performance.now();
        requestAnimationFrame(frame);
      } else if (!v) {
        enabled = false;
      }
    },
    setLayout,
    setProgress: (p) => {
      const v = Math.max(0, Math.min(1, p));
      if (v >= 1 && progress < 1) pulse = 1; // little burst when it finishes
      progress = v;
    },
  };
}
