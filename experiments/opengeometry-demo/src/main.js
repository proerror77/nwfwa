import "./styles.css";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import wasmUrl from "opengeometry/opengeometry_bg.wasm?url";
import { Cuboid, Cylinder, OpenGeometry, Sphere, Vector3 } from "opengeometry";

const stage = document.querySelector("#stage");
const wasmStatus = document.querySelector("#wasm-status");
const shapeCount = document.querySelector("#shape-count");
const pixelStatus = document.querySelector("#pixel-status");

const scene = new THREE.Scene();
scene.background = new THREE.Color(0xf4f7fb);

const camera = new THREE.PerspectiveCamera(45, 1, 0.1, 1000);
camera.position.set(7, 6, 9);

const renderer = new THREE.WebGLRenderer({
  antialias: true,
  preserveDrawingBuffer: true,
});
renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
renderer.shadowMap.enabled = true;
stage.appendChild(renderer.domElement);

const controls = new OrbitControls(camera, renderer.domElement);
controls.enableDamping = true;
controls.target.set(0, 0.5, 0);

scene.add(new THREE.HemisphereLight(0xffffff, 0x8aa0b8, 1.8));

const keyLight = new THREE.DirectionalLight(0xffffff, 2.8);
keyLight.position.set(5, 8, 6);
keyLight.castShadow = true;
scene.add(keyLight);

const grid = new THREE.GridHelper(12, 12, 0xb8c2cc, 0xd8dee7);
grid.position.y = -0.02;
scene.add(grid);

function resize() {
  const { clientWidth, clientHeight } = stage;
  camera.aspect = clientWidth / clientHeight;
  camera.updateProjectionMatrix();
  renderer.setSize(clientWidth, clientHeight, false);
}

function addKernelGeometry() {
  const base = new Cuboid({
    center: new Vector3(0, 0.45, 0),
    width: 5.6,
    height: 0.9,
    depth: 3.2,
    color: 0x2f80ed,
    fatOutlines: true,
    outlineWidth: 2,
  });
  base.castShadow = true;
  base.receiveShadow = true;

  const riskZone = new Cuboid({
    center: new Vector3(1.25, 1.18, -0.2),
    width: 1.5,
    height: 1.35,
    depth: 1.35,
    color: 0xef4444,
    fatOutlines: true,
    outlineWidth: 2,
  });
  riskZone.castShadow = true;

  const reviewTower = new Cylinder({
    center: new Vector3(-1.65, 1.15, 0.8),
    radius: 0.48,
    height: 1.8,
    segments: 32,
    angle: Math.PI * 2,
    color: 0xf59e0b,
    fatOutlines: true,
    outlineWidth: 2,
  });
  reviewTower.castShadow = true;

  const auditMarker = new Sphere({
    center: new Vector3(-0.7, 2.55, -0.85),
    radius: 0.36,
    widthSegments: 32,
    heightSegments: 16,
    color: 0x10b981,
    fatOutlines: true,
    outlineWidth: 2,
  });
  auditMarker.castShadow = true;

  scene.add(base, riskZone, reviewTower, auditMarker);
  return [base, riskZone, reviewTower, auditMarker];
}

function checkCanvasPixels() {
  const canvas = renderer.domElement;
  const context = canvas.getContext("webgl2") || canvas.getContext("webgl");
  if (!context || canvas.width === 0 || canvas.height === 0) {
    pixelStatus.textContent = "失败";
    pixelStatus.dataset.canvasState = "blank";
    return;
  }

  const pixels = new Uint8Array(canvas.width * canvas.height * 4);
  context.readPixels(0, 0, canvas.width, canvas.height, context.RGBA, context.UNSIGNED_BYTE, pixels);

  let nonBackgroundSamples = 0;
  const stride = Math.max(4, Math.floor(pixels.length / 4096));
  for (let i = 0; i < pixels.length; i += stride - (stride % 4)) {
    const r = pixels[i];
    const g = pixels[i + 1];
    const b = pixels[i + 2];
    if (Math.abs(r - 244) + Math.abs(g - 247) + Math.abs(b - 251) > 38) {
      nonBackgroundSamples += 1;
    }
  }

  const ok = nonBackgroundSamples > 24;
  pixelStatus.textContent = ok ? "通过" : "失败";
  pixelStatus.dataset.canvasState = ok ? "nonblank" : "blank";
  pixelStatus.dataset.nonBackgroundSamples = String(nonBackgroundSamples);
}

async function init() {
  resize();
  await OpenGeometry.create({ wasmURL: wasmUrl });
  wasmStatus.textContent = "已加载";

  const shapes = addKernelGeometry();
  shapeCount.textContent = String(shapes.length);

  const brepBytes = shapes
    .map((shape) => JSON.stringify(shape.getBrepData?.() ?? shape.getBrep?.() ?? {}).length)
    .reduce((sum, value) => sum + value, 0);
  wasmStatus.dataset.brepBytes = String(brepBytes);

  renderer.render(scene, camera);
  checkCanvasPixels();

  let frames = 0;
  function animate() {
    requestAnimationFrame(animate);
    frames += 1;
    scene.rotation.y += 0.0025;
    controls.update();
    renderer.render(scene, camera);
    if (frames === 20 && pixelStatus.dataset.canvasState !== "nonblank") {
      checkCanvasPixels();
    }
  }
  animate();
}

window.addEventListener("resize", resize);

init().catch((error) => {
  wasmStatus.textContent = "加载失败";
  pixelStatus.textContent = "失败";
  pixelStatus.dataset.canvasState = "error";
  console.error(error);
});
