import { useState } from 'react'
import { buildSkinUrl } from '../../api/skins'
import './PlayerHead.css'

export default function PlayerHead({ playerId, nickname, className = '' }: {
  playerId: string
  nickname: string
  className?: string
}) {
  const [failed, setFailed] = useState(false)

  if (failed) {
    return <span className={`player-head player-head--fallback ${className}`}>{nickname.slice(0, 2).toUpperCase()}</span>
  }

  const src = buildSkinUrl(playerId)
  return (
    <span className={`player-head ${className}`} role="img" aria-label={`Голова игрока ${nickname}`}>
      <img className="player-head__layer player-head__layer--base" src={src} alt="" draggable={false} onError={() => setFailed(true)} />
      <img className="player-head__layer player-head__layer--hat" src={src} alt="" draggable={false} />
    </span>
  )
}
