# OpenGeometry Demo

This is an isolated proof of concept for evaluating OpenGeometry in `nwfwa`.
It does not integrate with the Rust/Yew operations console.

The demo verifies:

- OpenGeometry WASM runtime loading through Vite.
- Kernel-backed geometry creation with `Cuboid`, `Cylinder`, and `Sphere`.
- Three.js rendering with a canvas pixel check to catch blank renders.

Run it locally:

```bash
npm install
npm run dev
```

Build and smoke-test:

```bash
npm run build
npm run smoke
```

The smoke script writes screenshots to `experiments/opengeometry-demo/artifacts/`,
which is ignored by the repository root `.gitignore`.

## Current Findings

- Production build succeeds with OpenGeometry 2.0.11.
- Built assets include a roughly 1.5 MB WASM file and a roughly 632 KB JavaScript
  chunk before gzip.
- `npm audit --audit-level=moderate` reports a moderate advisory through
  OpenGeometry's `uuid` dependency. Keep this experiment isolated until upstream
  ships a fixed dependency/build.
