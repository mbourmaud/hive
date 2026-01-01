import { useState } from 'react'
import Card from '../components/Card'

interface Product {
  id: number
  name: string
  description: string
  price: number
}

const PRODUCTS: Product[] = [
  { id: 1, name: 'Widget Pro', description: 'Professional-grade widget for all your needs', price: 99 },
  { id: 2, name: 'Gadget Plus', description: 'Enhanced gadget with premium features', price: 149 },
  { id: 3, name: 'Tool Master', description: 'The ultimate tool for productivity', price: 79 },
  { id: 4, name: 'Device Ultra', description: 'Ultra-powerful device for demanding tasks', price: 199 },
  { id: 5, name: 'Accessory Kit', description: 'Complete kit of essential accessories', price: 49 },
]

export default function Products() {
  const [search, setSearch] = useState('')

  const filteredProducts = PRODUCTS.filter(product =>
    product.name.toLowerCase().includes(search.toLowerCase()) ||
    product.description.toLowerCase().includes(search.toLowerCase())
  )

  return (
    <div className="page products">
      <h1>Products</h1>

      <div className="search-bar">
        <input
          type="text"
          placeholder="Search products..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="search-input"
        />
      </div>

      <div className="product-grid">
        {filteredProducts.length > 0 ? (
          filteredProducts.map(product => (
            <Card
              key={product.id}
              title={product.name}
              description={product.description}
              footer={`$${product.price}`}
            />
          ))
        ) : (
          <p className="no-results">No products found matching "{search}"</p>
        )}
      </div>
    </div>
  )
}
