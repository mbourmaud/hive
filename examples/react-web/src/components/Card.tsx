interface CardProps {
  title: string
  description: string
  footer?: string
  onClick?: () => void
}

export default function Card({ title, description, footer, onClick }: CardProps) {
  return (
    <div
      className={`card ${onClick ? 'card-clickable' : ''}`}
      onClick={onClick}
      role={onClick ? 'button' : undefined}
      tabIndex={onClick ? 0 : undefined}
    >
      <h3 className="card-title">{title}</h3>
      <p className="card-description">{description}</p>
      {footer && <div className="card-footer">{footer}</div>}
    </div>
  )
}
