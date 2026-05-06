import { useEffect, useRef, useState } from 'react'
import * as THREE from 'three'

import './GeckoModelViewer.css'

type Vec3 = [number, number, number]
type FaceName = 'north' | 'east' | 'south' | 'west' | 'up' | 'down'

interface GeckoFaceUv {
  uv: [number, number]
  uv_size?: [number, number]
}

interface GeckoCube {
  origin: Vec3
  size: Vec3
  pivot?: Vec3
  rotation?: Vec3
  inflate?: number
  uv?: Partial<Record<FaceName, GeckoFaceUv>> | [number, number]
}

interface GeckoBone {
  name: string
  parent?: string
  pivot?: Vec3
  rotation?: Vec3
  cubes?: GeckoCube[]
}

interface GeckoGeometry {
  description?: {
    identifier?: string
    texture_width?: number
    texture_height?: number
  }
  bones?: GeckoBone[]
}

interface GeckoModelFile {
  'minecraft:geometry'?: GeckoGeometry[]
}

export interface GeckoModelViewerProps {
  modelUrl: string
  textureUrl: string
  className?: string
  initialRotation?: Vec3
  initialZoom?: number
  mirrorVertical?: boolean
  verticalOffset?: number
  cameraDistance?: number
  background?: string | null
}

const geometryCache = new Map<string, Promise<THREE.BufferGeometry>>()
const textureCache = new Map<string, Promise<THREE.Texture>>()

const modelParserVersion = 'tacz-bedrock-preview-6'
const faceNames: FaceName[] = ['north', 'east', 'south', 'west', 'up', 'down']
const defaultInitialRotation: Vec3 = [0, 0, 0]

function degToRad(value: number) {
  return (value * Math.PI) / 180
}

function usesAttachmentAdapterAsDefaultStock(modelIdentifier: string) {
  const normalized = modelIdentifier.trim().toLowerCase()
  return normalized.includes('m4a1') || normalized.includes('rpk') || normalized.includes('sks_tactical')
}

function isHiddenPreviewBoneName(name: string, modelIdentifier: string) {
  const normalized = name.trim().toLowerCase()
  const allowDefaultStock = usesAttachmentAdapterAsDefaultStock(modelIdentifier)

  if (normalized === 'attachment_adapter') {
    return !allowDefaultStock
  }
  if (normalized.startsWith('oem_stock_')) {
    return !allowDefaultStock || normalized !== 'oem_stock_tactical'
  }
  if (normalized.startsWith('ar_stock_adapter')) {
    return true
  }

  return (
    normalized === 'camera'
    || normalized === 'constraint'
    || normalized === 'views'
    || normalized === 'fixed'
    || normalized === 'ground'
    || normalized === 'thirdperson_hand'
    || normalized === 'muzzle_flash'
    || normalized === 'shell'
    || normalized === 'mount'
    || normalized === 'rail2'
    || normalized === 'lefthand'
    || normalized === 'lefthand_pos'
    || normalized === 'righthand'
    || normalized === 'righthand_pos'
    || normalized === 'additional_magazine'
    || normalized.startsWith('bullet')
    || normalized.startsWith('762x')
    || normalized.startsWith('extd_mag')
    || normalized.startsWith('mag_extended')
    || normalized.endsWith('_pos')
    || normalized.endsWith('_view')
    || normalized.includes('refit_')
  )
}

function resolvePreviewVisibleBones(bones: GeckoBone[], modelIdentifier: string) {
  const byName = new Map(bones.map((bone) => [bone.name, bone]))
  const resolved = new Map<string, boolean>()

  const isVisible = (bone: GeckoBone): boolean => {
    const cached = resolved.get(bone.name)
    if (cached !== undefined) return cached

    const parent = bone.parent ? byName.get(bone.parent) : undefined
    const visible = !isHiddenPreviewBoneName(bone.name, modelIdentifier) && (!parent || isVisible(parent))
    resolved.set(bone.name, visible)
    return visible
  }

  for (const bone of bones) {
    isVisible(bone)
  }

  return resolved
}

function makePartMatrix(position: Vec3, rotation?: Vec3) {
  const matrix = new THREE.Matrix4().makeTranslation(position[0], position[1], position[2])
  if (!rotation) return matrix

  matrix.multiply(new THREE.Matrix4().makeRotationZ(degToRad(rotation[2])))
  matrix.multiply(new THREE.Matrix4().makeRotationY(degToRad(rotation[1])))
  matrix.multiply(new THREE.Matrix4().makeRotationX(degToRad(rotation[0])))
  return matrix
}

function convertBonePivot(bone: GeckoBone, parent?: GeckoBone): Vec3 {
  const pivot = bone.pivot ?? [0, 0, 0]
  if (!parent) return [pivot[0], 24 - pivot[1], pivot[2]]

  const parentPivot = parent.pivot ?? [0, 0, 0]
  return [
    pivot[0] - parentPivot[0],
    parentPivot[1] - pivot[1],
    pivot[2] - parentPivot[2],
  ]
}

function convertCubePivot(parent: GeckoBone, cube: GeckoCube): Vec3 {
  const parentPivot = parent.pivot ?? [0, 0, 0]
  const cubePivot = cube.pivot ?? parentPivot
  return [
    cubePivot[0] - parentPivot[0],
    parentPivot[1] - cubePivot[1],
    cubePivot[2] - parentPivot[2],
  ]
}

function convertCubeOrigin(parent: GeckoBone, cube: GeckoCube): Vec3 {
  const pivot = parent.pivot ?? [0, 0, 0]
  return [
    cube.origin[0] - pivot[0],
    pivot[1] - cube.origin[1] - cube.size[1],
    cube.origin[2] - pivot[2],
  ]
}

function convertRotatedCubeOrigin(cube: GeckoCube): Vec3 {
  const pivot = cube.pivot ?? [0, 0, 0]
  return [
    cube.origin[0] - pivot[0],
    pivot[1] - cube.origin[1] - cube.size[1],
    cube.origin[2] - pivot[2],
  ]
}

function resolveBoneTransforms(bones: GeckoBone[]) {
  const byName = new Map(bones.map((bone) => [bone.name, bone]))
  const resolved = new Map<string, THREE.Matrix4>()

  const resolve = (bone: GeckoBone): THREE.Matrix4 => {
    const cached = resolved.get(bone.name)
    if (cached) return cached

    const parent = bone.parent ? byName.get(bone.parent) : undefined
    const matrix = parent ? resolve(parent).clone() : new THREE.Matrix4()
    matrix.multiply(makePartMatrix(convertBonePivot(bone, parent), bone.rotation))
    resolved.set(bone.name, matrix)
    return matrix
  }

  for (const bone of bones) {
    resolve(bone)
  }

  return resolved
}

function pushFace(
  positions: number[],
  uvs: number[],
  indices: number[],
  vertices: THREE.Vector3[],
  faceUv: GeckoFaceUv | undefined,
  textureWidth: number,
  textureHeight: number,
  transform: THREE.Matrix4,
) {
  if (!faceUv) return

  const baseIndex = positions.length / 3
  for (const vertex of vertices) {
    const transformed = vertex.clone().applyMatrix4(transform)
    positions.push(transformed.x, transformed.y, transformed.z)
  }

  const [u, v] = faceUv.uv
  const [uvWidth, uvHeight] = faceUv.uv_size ?? [0, 0]
  const u1 = u / textureWidth
  const u2 = (u + uvWidth) / textureWidth
  const v1 = v / textureHeight
  const v2 = (v + uvHeight) / textureHeight

  uvs.push(u2, v1, u1, v1, u1, v2, u2, v2)
  indices.push(baseIndex, baseIndex + 1, baseIndex + 2, baseIndex, baseIndex + 2, baseIndex + 3)
}

function pushCube(
  cube: GeckoCube,
  localOrigin: Vec3,
  transform: THREE.Matrix4,
  textureWidth: number,
  textureHeight: number,
  positions: number[],
  uvs: number[],
  indices: number[],
) {
  if (!cube.uv || Array.isArray(cube.uv)) return

  const inflate = cube.inflate ?? 0
  const minX = localOrigin[0] - inflate
  const minY = localOrigin[1] - inflate
  const minZ = localOrigin[2] - inflate
  const maxX = localOrigin[0] + cube.size[0] + inflate
  const maxY = localOrigin[1] + cube.size[1] + inflate
  const maxZ = localOrigin[2] + cube.size[2] + inflate

  const vertex1 = new THREE.Vector3(minX, minY, minZ)
  const vertex2 = new THREE.Vector3(maxX, minY, minZ)
  const vertex3 = new THREE.Vector3(maxX, maxY, minZ)
  const vertex4 = new THREE.Vector3(minX, maxY, minZ)
  const vertex5 = new THREE.Vector3(minX, minY, maxZ)
  const vertex6 = new THREE.Vector3(maxX, minY, maxZ)
  const vertex7 = new THREE.Vector3(maxX, maxY, maxZ)
  const vertex8 = new THREE.Vector3(minX, maxY, maxZ)

  const faceVertices: Record<FaceName, THREE.Vector3[]> = {
    up: [vertex6, vertex5, vertex1, vertex2],
    down: [vertex3, vertex4, vertex8, vertex7],
    east: [vertex1, vertex5, vertex8, vertex4],
    north: [vertex2, vertex1, vertex4, vertex3],
    west: [vertex6, vertex2, vertex3, vertex7],
    south: [vertex5, vertex6, vertex7, vertex8],
  }

  for (const faceName of faceNames) {
    pushFace(positions, uvs, indices, faceVertices[faceName], cube.uv[faceName], textureWidth, textureHeight, transform)
  }
}

async function loadGeometry(modelUrl: string) {
  const cacheKey = `${modelParserVersion}:${modelUrl}`
  const cached = geometryCache.get(cacheKey)
  if (cached) return cached

  const promise = fetch(modelUrl)
    .then((response) => {
      if (!response.ok) throw new Error(`Failed to load model: ${response.status}`)
      return response.json() as Promise<GeckoModelFile>
    })
    .then((file) => {
      const geometryData = file['minecraft:geometry']?.[0]
      if (!geometryData) throw new Error('Unsupported GeckoLib geometry file')

      const textureWidth = geometryData.description?.texture_width ?? 64
      const textureHeight = geometryData.description?.texture_height ?? 64
      const modelIdentifier = geometryData.description?.identifier ?? ''
      const bones = geometryData.bones ?? []
      const positions: number[] = []
      const uvs: number[] = []
      const indices: number[] = []
      const visibleBones = resolvePreviewVisibleBones(bones, modelIdentifier)
      const boneTransforms = resolveBoneTransforms(bones)

      for (const bone of bones) {
        if (!visibleBones.get(bone.name)) continue
        const boneTransform = boneTransforms.get(bone.name) ?? new THREE.Matrix4()
        for (const cube of bone.cubes ?? []) {
          const cubeTransform = cube.rotation
            ? boneTransform.clone().multiply(makePartMatrix(convertCubePivot(bone, cube), cube.rotation))
            : boneTransform
          const cubeOrigin = cube.rotation ? convertRotatedCubeOrigin(cube) : convertCubeOrigin(bone, cube)
          pushCube(cube, cubeOrigin, cubeTransform, textureWidth, textureHeight, positions, uvs, indices)
        }
      }

      const geometry = new THREE.BufferGeometry()
      geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3))
      geometry.setAttribute('uv', new THREE.Float32BufferAttribute(uvs, 2))
      geometry.setIndex(indices)
      geometry.applyMatrix4(new THREE.Matrix4().makeScale(-1, -1, 1))
      geometry.computeVertexNormals()
      geometry.computeBoundingBox()
      geometry.computeBoundingSphere()
      geometry.center()

      return geometry
    })

  geometryCache.set(cacheKey, promise)
  return promise
}

async function loadTexture(textureUrl: string) {
  const cached = textureCache.get(textureUrl)
  if (cached) return cached

  const promise = new Promise<THREE.Texture>((resolve, reject) => {
    new THREE.TextureLoader().load(textureUrl, resolve, undefined, reject)
  }).then((texture) => {
    texture.colorSpace = THREE.SRGBColorSpace
    texture.magFilter = THREE.NearestFilter
    texture.minFilter = THREE.NearestFilter
    texture.generateMipmaps = false
    texture.wrapS = THREE.ClampToEdgeWrapping
    texture.wrapT = THREE.ClampToEdgeWrapping
    texture.flipY = false
    texture.needsUpdate = true
    return texture
  })

  textureCache.set(textureUrl, promise)
  return promise
}

export default function GeckoModelViewer({
  modelUrl,
  textureUrl,
  className,
  initialRotation = defaultInitialRotation,
  initialZoom = 1,
  mirrorVertical = false,
  verticalOffset = 0,
  cameraDistance,
  background = null,
}: GeckoModelViewerProps) {
  const rootRef = useRef<HTMLDivElement>(null)
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const [error, setError] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(true)

  useEffect(() => {
    const canvas = canvasRef.current
    const root = rootRef.current
    if (!canvas || !root) return undefined

    let disposed = false
    let renderQueued = false
    const [initialRotationX, initialRotationY, initialRotationZ] = initialRotation
    const rotation = { x: degToRad(initialRotationX), y: degToRad(initialRotationY) }
    const zoom = { value: initialZoom }
    const fitRadius = { value: cameraDistance ?? 24 }
    const activePointers = new Map<number, { x: number; y: number; time: number }>()
    let pinchDistance: number | null = null
    let horizontalVelocity = 0
    let inertiaFrame = 0
    let lastInertiaTime = 0
    let autoRotationFrame = 0
    let lastAutoRotationTime = 0

    const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: background === null })
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
    renderer.outputColorSpace = THREE.SRGBColorSpace
    if (background !== null) {
      renderer.setClearColor(background)
    }

    const scene = new THREE.Scene()
    const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.1, 5000)
    const modelGroup = new THREE.Group()
    modelGroup.rotation.z = degToRad(initialRotationZ)
    scene.add(modelGroup)

    scene.add(new THREE.HemisphereLight(0xffffff, 0x262b36, 1.35))
    const keyLight = new THREE.DirectionalLight(0xffffff, 2.15)
    keyLight.position.set(3, 5, 6)
    scene.add(keyLight)
    const rimLight = new THREE.DirectionalLight(0xa9bbff, 0.9)
    rimLight.position.set(-5, 2, -4)
    scene.add(rimLight)
    const fillLight = new THREE.DirectionalLight(0xffd5a8, 0.42)
    fillLight.position.set(-2, -3, 3)
    scene.add(fillLight)

    const requestRender = () => {
      if (renderQueued || disposed) return
      renderQueued = true
      window.requestAnimationFrame(() => {
        renderQueued = false
        if (disposed) return
        modelGroup.rotation.x = rotation.x
        modelGroup.rotation.y = rotation.y
        modelGroup.scale.y = mirrorVertical ? -1 : 1
        modelGroup.position.y = fitRadius.value * verticalOffset
        camera.position.set(0, 0, 1000)
        camera.zoom = zoom.value
        camera.updateProjectionMatrix()
        camera.lookAt(0, 0, 0)
        renderer.render(scene, camera)
      })
    }

    const stopInertia = () => {
      horizontalVelocity = 0
      lastInertiaTime = 0
      if (inertiaFrame) {
        window.cancelAnimationFrame(inertiaFrame)
        inertiaFrame = 0
      }
    }

    const stepInertia = (time: number) => {
      if (disposed || activePointers.size > 0) {
        inertiaFrame = 0
        return
      }

      const elapsed = Math.min(time - lastInertiaTime, 32)
      lastInertiaTime = time
      rotation.y += horizontalVelocity * elapsed
      horizontalVelocity *= Math.pow(0.92, elapsed / 16.67)
      requestRender()

      if (Math.abs(horizontalVelocity) > 0.00002) {
        inertiaFrame = window.requestAnimationFrame(stepInertia)
      } else {
        stopInertia()
      }
    }

    const startInertia = () => {
      if (Math.abs(horizontalVelocity) <= 0.00002 || inertiaFrame) return
      lastInertiaTime = performance.now()
      inertiaFrame = window.requestAnimationFrame(stepInertia)
    }

    const stepAutoRotation = (time: number) => {
      if (disposed) {
        autoRotationFrame = 0
        return
      }

      const elapsed = lastAutoRotationTime ? Math.min(time - lastAutoRotationTime, 32) : 16.67
      lastAutoRotationTime = time

      if (activePointers.size === 0 && !inertiaFrame) {
        rotation.y += elapsed * 0.00022
        requestRender()
      }

      autoRotationFrame = window.requestAnimationFrame(stepAutoRotation)
    }

    const resize = () => {
      const rect = root.getBoundingClientRect()
      const width = Math.max(1, Math.floor(rect.width))
      const height = Math.max(1, Math.floor(rect.height))
      renderer.setSize(width, height, false)
      const aspect = width / height
      camera.left = -fitRadius.value * aspect
      camera.right = fitRadius.value * aspect
      camera.top = fitRadius.value
      camera.bottom = -fitRadius.value
      camera.updateProjectionMatrix()
      requestRender()
    }

    const resizeObserver = new ResizeObserver(resize)
    resizeObserver.observe(root)

    const onWheel = (event: WheelEvent) => {
      event.preventDefault()
      zoom.value = THREE.MathUtils.clamp(zoom.value * (1 - event.deltaY * 0.001), 0.35, 8)
      requestRender()
    }

    const onPointerDown = (event: PointerEvent) => {
      stopInertia()
      canvas.setPointerCapture(event.pointerId)
      activePointers.set(event.pointerId, { x: event.clientX, y: event.clientY, time: performance.now() })
      pinchDistance = null
    }

    const onPointerMove = (event: PointerEvent) => {
      const pointer = activePointers.get(event.pointerId)
      if (!pointer) return

      const previous = { ...pointer }
      const time = performance.now()
      pointer.x = event.clientX
      pointer.y = event.clientY
      pointer.time = time

      if (activePointers.size === 1) {
        const deltaRotation = (pointer.x - previous.x) * 0.01
        const elapsed = Math.max(time - previous.time, 1)
        rotation.y += deltaRotation
        horizontalVelocity = deltaRotation / elapsed
        requestRender()
        return
      }

      if (activePointers.size === 2) {
        horizontalVelocity = 0
        const pointers = Array.from(activePointers.values())
        const firstPointer = pointers[0]
        const secondPointer = pointers[1]
        if (!firstPointer || !secondPointer) return
        const distance = Math.hypot(firstPointer.x - secondPointer.x, firstPointer.y - secondPointer.y)
        if (pinchDistance !== null) {
          zoom.value = THREE.MathUtils.clamp(zoom.value * (distance / pinchDistance), 0.35, 8)
          requestRender()
        }
        pinchDistance = distance
      }
    }

    const onPointerEnd = (event: PointerEvent) => {
      activePointers.delete(event.pointerId)
      pinchDistance = null
      if (activePointers.size === 0) {
        startInertia()
      } else {
        horizontalVelocity = 0
      }
    }

    canvas.addEventListener('wheel', onWheel, { passive: false })
    canvas.addEventListener('pointerdown', onPointerDown)
    canvas.addEventListener('pointermove', onPointerMove)
    canvas.addEventListener('pointerup', onPointerEnd)
    canvas.addEventListener('pointercancel', onPointerEnd)

    Promise.all([loadGeometry(modelUrl), loadTexture(textureUrl)])
      .then(([geometry, texture]) => {
        if (disposed) return
        const material = new THREE.MeshLambertMaterial({
          map: texture,
          transparent: true,
          alphaTest: 0.1,
          side: THREE.DoubleSide,
        })
        const mesh = new THREE.Mesh(geometry, material)
        const radius = geometry.boundingSphere?.radius ?? 16
        fitRadius.value = cameraDistance ?? Math.max(radius * 1.25, 1)
        zoom.value = initialZoom
        modelGroup.add(mesh)
        setError(null)
        setIsLoading(false)
        resize()
        requestRender()
        if (!autoRotationFrame) {
          autoRotationFrame = window.requestAnimationFrame(stepAutoRotation)
        }
      })
      .catch((reason: unknown) => {
        if (disposed) return
        setIsLoading(false)
        setError(reason instanceof Error ? reason.message : 'Failed to load model')
      })

    resize()

    return () => {
      disposed = true
      resizeObserver.disconnect()
      canvas.removeEventListener('wheel', onWheel)
      canvas.removeEventListener('pointerdown', onPointerDown)
      canvas.removeEventListener('pointermove', onPointerMove)
      canvas.removeEventListener('pointerup', onPointerEnd)
      canvas.removeEventListener('pointercancel', onPointerEnd)
      if (inertiaFrame) {
        window.cancelAnimationFrame(inertiaFrame)
      }
      if (autoRotationFrame) {
        window.cancelAnimationFrame(autoRotationFrame)
      }
      for (const child of modelGroup.children) {
        const mesh = child as THREE.Mesh<THREE.BufferGeometry, THREE.Material>
        mesh.material.dispose()
      }
      renderer.dispose()
    }
  }, [
    background,
    cameraDistance,
    initialRotation[0],
    initialRotation[1],
    initialRotation[2],
    initialZoom,
    mirrorVertical,
    modelUrl,
    textureUrl,
    verticalOffset,
  ])

  const classes = ['gecko-model-viewer', className].filter(Boolean).join(' ')

  return (
    <div ref={rootRef} className={classes}>
      <canvas ref={canvasRef} />
      {!error && <div className="gecko-model-viewer__hint">Потяните влево или вправо, чтобы повернуть</div>}
      {isLoading && <div className="gecko-model-viewer__state">Loading model</div>}
      {error && <div className="gecko-model-viewer__state gecko-model-viewer__state_error">{error}</div>}
    </div>
  )
}
