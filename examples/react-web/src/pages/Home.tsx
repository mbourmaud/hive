import { Link } from 'react-router-dom'
import Button from '../components/Button'

export default function Home() {
  return (
    <div className="page home">
      <section className="hero">
        <h1>Welcome to React Web Example</h1>
        <p>A demo app for testing with Hive drones and Playwright</p>
        <div className="hero-actions">
          <Link to="/products">
            <Button>View Products</Button>
          </Link>
          <Link to="/contact">
            <Button variant="secondary">Contact Us</Button>
          </Link>
        </div>
      </section>

      <section className="features">
        <h2>Features</h2>
        <div className="feature-grid">
          <div className="feature-card">
            <h3>React Router</h3>
            <p>Multi-page navigation with client-side routing</p>
          </div>
          <div className="feature-card">
            <h3>Playwright Testing</h3>
            <p>Autonomous browser testing via Hive drones</p>
          </div>
          <div className="feature-card">
            <h3>TypeScript</h3>
            <p>Type-safe development with full IDE support</p>
          </div>
        </div>
      </section>
    </div>
  )
}
