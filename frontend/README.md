# MatterFlow frontend

React + TypeScript, generated with the official Vite react-ts template.

Run commands from the repository root:

~~~powershell
npm.cmd install
npm.cmd run dev
npm.cmd run lint
npm.cmd run build
~~~

The development URL is http://localhost:5173. Requests to /api are proxied to the Rust service at http://127.0.0.1:3000.

For deployment, set VITE_API_BASE_URL to the public Rust API origin before building. The build output is frontend/dist. VITE_ values are public; do not put secrets in them.

src/features contains feature-specific components, src/components is for reusable UI, and src/lib contains the shared API client. Only the health connection feature exists at this stage.
