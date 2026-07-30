import { useState, type CSSProperties } from 'react'
import Pride from 'react-canvas-confetti/dist/presets/pride'
import './PaymentImageConfetti.css'

const confettiImageModules = import.meta.glob(
  '../assets/payment-confetti/*.{png,jpg,jpeg,webp,gif,svg}',
  {
    eager: true,
    import: 'default',
    query: '?url',
  },
) as Record<string, string>

const confettiImages = Object.values(confettiImageModules)
const PARTICLE_COUNT = 100

type ConfettiParticle = {
  id: number
  imageUrl: string
  style: CSSProperties & Record<`--${string}`, string>
}

function randomBetween(min: number, max: number) {
  return min + Math.random() * (max - min)
}

function createParticles(): ConfettiParticle[] {
  return Array.from({ length: PARTICLE_COUNT }, (_, id) => ({
    id,
    imageUrl: confettiImages[id % confettiImages.length],
    style: {
      '--confetti-left': `${randomBetween(0, 100).toFixed(2)}vw`,
      '--confetti-size': `${randomBetween(44, 74).toFixed(0)}px`,
      '--confetti-delay': `${randomBetween(0, 1.2).toFixed(2)}s`,
      '--confetti-duration': `${randomBetween(3.6, 6.2).toFixed(2)}s`,
      '--confetti-drift': `${randomBetween(-18, 18).toFixed(2)}vw`,
      '--confetti-rotation': `${randomBetween(540, 1260).toFixed(0)}deg`,
    },
  }))
}

export default function PaymentImageConfetti() {
  const [particles] = useState(createParticles)

  if (confettiImages.length === 0) {
    return (
      <Pride
        style={{
          position: 'fixed',
          pointerEvents: 'none',
          width: '100%',
          height: '100%',
          top: 0,
          left: 0,
          zIndex: 10,
        }}
        autorun={{ speed: 1 }}
        decorateOptions={(defaultOptions) => ({
          ...defaultOptions,
          particleCount: PARTICLE_COUNT,
          spread: 90,
          zIndex: 10,
          colors: ['#26ccff', '#a25afd', '#ff5e7e', '#88ff5a', '#fcff42', '#ffa62d', '#ff36ff'],
        })}
      />
    )
  }

  return (
    <div className="payment-image-confetti" aria-hidden="true">
      {particles.map((particle) => (
        <img
          className="payment-image-confetti__particle"
          key={particle.id}
          src={particle.imageUrl}
          alt=""
          style={particle.style}
        />
      ))}
    </div>
  )
}
