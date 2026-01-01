import { NavLink } from 'react-router-dom'

export default function Header() {
  return (
    <header className="header">
      <div className="header-content">
        <NavLink to="/" className="logo">
          React Web Example
        </NavLink>
        <nav className="nav">
          <NavLink to="/" className={({ isActive }) => isActive ? 'nav-link active' : 'nav-link'}>
            Home
          </NavLink>
          <NavLink to="/products" className={({ isActive }) => isActive ? 'nav-link active' : 'nav-link'}>
            Products
          </NavLink>
          <NavLink to="/contact" className={({ isActive }) => isActive ? 'nav-link active' : 'nav-link'}>
            Contact
          </NavLink>
        </nav>
      </div>
    </header>
  )
}
