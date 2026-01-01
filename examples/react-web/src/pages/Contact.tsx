import { useState, FormEvent } from 'react'
import Button from '../components/Button'

export default function Contact() {
  const [name, setName] = useState('')
  const [email, setEmail] = useState('')
  const [message, setMessage] = useState('')
  const [submitted, setSubmitted] = useState(false)

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault()
    console.log('Form submitted:', { name, email, message })
    setSubmitted(true)
  }

  if (submitted) {
    return (
      <div className="page contact">
        <div className="success-message">
          <h1>Thank You!</h1>
          <p>Your message has been sent successfully.</p>
          <Button onClick={() => setSubmitted(false)}>Send Another Message</Button>
        </div>
      </div>
    )
  }

  return (
    <div className="page contact">
      <h1>Contact Us</h1>
      <p>Have a question? Send us a message and we'll get back to you.</p>

      <form className="contact-form" onSubmit={handleSubmit}>
        <div className="form-group">
          <label htmlFor="name">Name</label>
          <input
            type="text"
            id="name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Your name"
            required
          />
        </div>

        <div className="form-group">
          <label htmlFor="email">Email</label>
          <input
            type="email"
            id="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="your@email.com"
            required
          />
        </div>

        <div className="form-group">
          <label htmlFor="message">Message</label>
          <textarea
            id="message"
            value={message}
            onChange={(e) => setMessage(e.target.value)}
            placeholder="Your message..."
            rows={5}
            required
          />
        </div>

        <Button type="submit">Send Message</Button>
      </form>
    </div>
  )
}
