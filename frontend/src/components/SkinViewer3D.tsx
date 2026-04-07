import { useEffect, useRef } from 'react'
import { SkinViewer, WalkingAnimation } from 'skinview3d'

import './SkinViewer3D.css'

interface SkinViewer3DProps {
  skinUrl: string
  model?: 'default' | 'slim'
  width?: number
  height?: number
}

export default function SkinViewer3D({
  skinUrl,
  model = 'default',
  width = 300,
  height = 400,
}: SkinViewer3DProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const viewerRef = useRef<SkinViewer | null>(null)

  useEffect(() => {
    if (!canvasRef.current) return

    const viewer = new SkinViewer({
      canvas: canvasRef.current,
      width,
      height,
      skin: skinUrl,
      model: model === 'slim' ? 'slim' : 'default',
    })

    viewer.animation = new WalkingAnimation()
    viewer.animation.speed = 0.5

    viewer.camera.rotation.x = -0.5
    viewer.camera.rotation.y = 0.5
    viewer.camera.rotation.z = 0
    viewer.camera.position.x = 0
    viewer.camera.position.y = 18
    viewer.camera.position.z = 50

    viewer.zoom = 0.9

    viewer.controls.enableRotate = true
    viewer.controls.enableZoom = true
    viewer.controls.enablePan = false

    viewerRef.current = viewer

    return () => {
      viewer.dispose()
    }
  }, [skinUrl, model, width, height])

  return (
    <div className="skin-viewer-3d">
      <canvas ref={canvasRef} />
      <div className="viewer-hint">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <circle cx="12" cy="12" r="10" />
          <path d="M12 8v4M12 16h.01" />
        </svg>
        Перетащите для вращения
      </div>
    </div>
  )
}
