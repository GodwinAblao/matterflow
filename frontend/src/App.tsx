import { ApiStatus } from './features/health/ApiStatus'
import './App.css'

export default function App() {
  return (
    <main className="shell">
      <header className="brand"><span className="brand-mark" aria-hidden="true">M</span> MatterFlow</header>
      <div className="intro">
        <p className="eyebrow">PROJECT FOUNDATION</p>
        <h1>Clear requirements.<br />Confident next steps.</h1>
        <p className="summary">Legal document readiness and operations analytics, built with Rust.</p>
        <ApiStatus />
      </div>
      <section className="modules" aria-label="Planned modules">
        <article><span>01</span><h2>Matters</h2><p>A clear view of assignments and progress.</p></article>
        <article><span>02</span><h2>Document readiness</h2><p>Track requirements from receipt to review.</p></article>
        <article><span>03</span><h2>Operations analytics</h2><p>Understand outstanding work and follow-ups.</p></article>
      </section>
      <footer>Framework setup only. Workflow features are planned.<br />Independent portfolio prototype. Future demonstration records will be fictional.</footer>
    </main>
  )
}
