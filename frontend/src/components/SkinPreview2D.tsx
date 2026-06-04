import { useEffect, useRef } from "react";

type Props = {
  src: string;
  alt: string;
  className?: string;
  onError?: () => void;
};

type Region = {
  sx: number;
  sy: number;
  sw: number;
  sh: number;
};

type Vec3 = {
  x: number;
  y: number;
  z: number;
};

type Vec2 = {
  x: number;
  y: number;
};

type TextureSource = {
  data: Uint8ClampedArray;
  width: number;
  height: number;
};

type Face = {
  region: Region;
  points3d: [Vec3, Vec3, Vec3, Vec3];
  shade?: string;
};

type CuboidTextures = {
  front: Region;
  side: Region;
  top: Region;
};

const COS_30 = Math.cos(Math.PI / 6);
const SIN_30 = Math.sin(Math.PI / 6);

function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.decoding = "async";
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error("Failed to load skin image"));
    image.src = src;
  });
}

function project(point: Vec3, scale: number, origin: Vec2): Vec2 {
  return {
    x: origin.x + (point.z - point.x) * COS_30 * scale,
    y: origin.y + ((point.x + point.z) * SIN_30 - point.y) * scale,
  };
}

function quadBounds(points: Vec2[]) {
  let minX = Infinity;
  let maxX = -Infinity;
  let minY = Infinity;
  let maxY = -Infinity;

  for (const point of points) {
    minX = Math.min(minX, point.x);
    maxX = Math.max(maxX, point.x);
    minY = Math.min(minY, point.y);
    maxY = Math.max(maxY, point.y);
  }

  return { minX, maxX, minY, maxY };
}

function lerpPoint(a: Vec2, b: Vec2, t: number): Vec2 {
  return {
    x: a.x + (b.x - a.x) * t,
    y: a.y + (b.y - a.y) * t,
  };
}

function bilerpQuad(points: [Vec2, Vec2, Vec2, Vec2], u: number, v: number): Vec2 {
  const top = lerpPoint(points[0], points[1], u);
  const bottom = lerpPoint(points[3], points[2], u);
  return lerpPoint(top, bottom, v);
}

function expandPointFromCenter(point: Vec2, center: Vec2, amount: number): Vec2 {
  const dx = point.x - center.x;
  const dy = point.y - center.y;
  const length = Math.hypot(dx, dy) || 1;
  return {
    x: point.x + (dx / length) * amount,
    y: point.y + (dy / length) * amount,
  };
}

function expandQuad(points: [Vec2, Vec2, Vec2, Vec2], amount: number): [Vec2, Vec2, Vec2, Vec2] {
  const center = points.reduce(
    (acc, point) => ({ x: acc.x + point.x / 4, y: acc.y + point.y / 4 }),
    { x: 0, y: 0 },
  );

  return points.map((point) => expandPointFromCenter(point, center, amount)) as [
    Vec2,
    Vec2,
    Vec2,
    Vec2,
  ];
}

function drawTexturedQuad(
  ctx: CanvasRenderingContext2D,
  texture: TextureSource,
  region: Region,
  points: [Vec2, Vec2, Vec2, Vec2],
  shade?: string,
) {
  for (let py = 0; py < region.sh; py += 1) {
    for (let px = 0; px < region.sw; px += 1) {
      const sampleX = region.sx + px;
      const sampleY = region.sy + py;
      const offset = (sampleY * texture.width + sampleX) * 4;
      const alpha = texture.data[offset + 3];
      if (alpha === 0) continue;

      const u0 = px / region.sw;
      const v0 = py / region.sh;
      const u1 = (px + 1) / region.sw;
      const v1 = (py + 1) / region.sh;

      const quad = [
        bilerpQuad(points, u0, v0),
        bilerpQuad(points, u1, v0),
        bilerpQuad(points, u1, v1),
        bilerpQuad(points, u0, v1),
      ] as [Vec2, Vec2, Vec2, Vec2];
      const expandedQuad = expandQuad(quad, 0.35);

      ctx.save();
      ctx.beginPath();
      ctx.moveTo(expandedQuad[0].x, expandedQuad[0].y);
      ctx.lineTo(expandedQuad[1].x, expandedQuad[1].y);
      ctx.lineTo(expandedQuad[2].x, expandedQuad[2].y);
      ctx.lineTo(expandedQuad[3].x, expandedQuad[3].y);
      ctx.closePath();
      ctx.fillStyle = `rgba(${texture.data[offset]}, ${texture.data[offset + 1]}, ${texture.data[offset + 2]}, ${alpha / 255})`;
      ctx.fill();
      ctx.restore();
    }
  }

  if (shade) {
    ctx.save();
    ctx.beginPath();
    ctx.moveTo(points[0].x, points[0].y);
    ctx.lineTo(points[1].x, points[1].y);
    ctx.lineTo(points[2].x, points[2].y);
    ctx.lineTo(points[3].x, points[3].y);
    ctx.closePath();
    ctx.fillStyle = shade;
    ctx.fill();
    ctx.restore();
  }
}

function cuboidFaces(
  x: number,
  y: number,
  z: number,
  width: number,
  height: number,
  depth: number,
  textures: CuboidTextures,
): Face[] {
  const x0 = x;
  const x1 = x + width;
  const y0 = y;
  const y1 = y + height;
  const z0 = z;
  const z1 = z + depth;

  return [
    {
      region: textures.top,
      points3d: [
        { x: x0, y: y1, z: z0 },
        { x: x1, y: y1, z: z0 },
        { x: x1, y: y1, z: z1 },
        { x: x0, y: y1, z: z1 },
      ],
      shade: "rgba(255,255,255,0.06)",
    },
    {
      region: textures.side,
      points3d: [
        { x: x1, y: y1, z: z0 },
        { x: x1, y: y1, z: z1 },
        { x: x1, y: y0, z: z1 },
        { x: x1, y: y0, z: z0 },
      ],
      shade: "rgba(0,0,0,0.14)",
    },
    {
      region: textures.front,
      points3d: [
        { x: x0, y: y1, z: z1 },
        { x: x1, y: y1, z: z1 },
        { x: x1, y: y0, z: z1 },
        { x: x0, y: y0, z: z1 },
      ],
    },
  ];
}

function buildFaces(image: HTMLImageElement) {
  const hasOuterLayer = image.height >= 64;

  const faces: Face[] = [];

  faces.push(
    ...cuboidFaces(-4, 24, -4, 8, 8, 8, {
      front: { sx: 8, sy: 8, sw: 8, sh: 8 },
      side: { sx: 0, sy: 8, sw: 8, sh: 8 },
      top: { sx: 8, sy: 0, sw: 8, sh: 8 },
    }),
  );

  if (hasOuterLayer) {
    faces.push(
      ...cuboidFaces(-4.25, 23.75, -4.25, 8.5, 8.5, 8.5, {
        front: { sx: 40, sy: 8, sw: 8, sh: 8 },
        side: { sx: 32, sy: 8, sw: 8, sh: 8 },
        top: { sx: 40, sy: 0, sw: 8, sh: 8 },
      }),
    );
  }

  faces.push(
    ...cuboidFaces(-4, 12, -2, 8, 12, 4, {
      front: { sx: 20, sy: 20, sw: 8, sh: 12 },
      side: { sx: 16, sy: 20, sw: 4, sh: 12 },
      top: { sx: 20, sy: 16, sw: 8, sh: 4 },
    }),
  );

  faces.push(
    ...cuboidFaces(-8, 12, -2, 4, 12, 4, {
      front: hasOuterLayer ? { sx: 36, sy: 52, sw: 4, sh: 12 } : { sx: 44, sy: 20, sw: 4, sh: 12 },
      side: hasOuterLayer ? { sx: 32, sy: 52, sw: 4, sh: 12 } : { sx: 40, sy: 20, sw: 4, sh: 12 },
      top: hasOuterLayer ? { sx: 36, sy: 48, sw: 4, sh: 4 } : { sx: 44, sy: 16, sw: 4, sh: 4 },
    }),
  );

  faces.push(
    ...cuboidFaces(4, 12, -2, 4, 12, 4, {
      front: { sx: 44, sy: 20, sw: 4, sh: 12 },
      side: { sx: 40, sy: 20, sw: 4, sh: 12 },
      top: { sx: 44, sy: 16, sw: 4, sh: 4 },
    }),
  );

  faces.push(
    ...cuboidFaces(-4, 0, -2, 4, 12, 4, {
      front: hasOuterLayer ? { sx: 20, sy: 52, sw: 4, sh: 12 } : { sx: 4, sy: 20, sw: 4, sh: 12 },
      side: hasOuterLayer ? { sx: 16, sy: 52, sw: 4, sh: 12 } : { sx: 0, sy: 20, sw: 4, sh: 12 },
      top: hasOuterLayer ? { sx: 20, sy: 48, sw: 4, sh: 4 } : { sx: 4, sy: 16, sw: 4, sh: 4 },
    }),
  );

  faces.push(
    ...cuboidFaces(0, 0, -2, 4, 12, 4, {
      front: { sx: 4, sy: 20, sw: 4, sh: 12 },
      side: { sx: 0, sy: 20, sw: 4, sh: 12 },
      top: { sx: 4, sy: 16, sw: 4, sh: 4 },
    }),
  );

  return faces;
}

function renderSkinPreview(canvas: HTMLCanvasElement, image: HTMLImageElement) {
  const ratio = window.devicePixelRatio || 1;
  const cssWidth = canvas.clientWidth || 220;
  const cssHeight = canvas.clientHeight || 220;

  canvas.width = Math.max(1, Math.round(cssWidth * ratio));
  canvas.height = Math.max(1, Math.round(cssHeight * ratio));

  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  const textureCanvas = document.createElement("canvas");
  textureCanvas.width = image.naturalWidth;
  textureCanvas.height = image.naturalHeight;
  const textureCtx = textureCanvas.getContext("2d", { willReadFrequently: true });
  if (!textureCtx) return;
  textureCtx.imageSmoothingEnabled = false;
  textureCtx.drawImage(image, 0, 0);
  const texture = textureCtx.getImageData(0, 0, textureCanvas.width, textureCanvas.height);

  ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
  ctx.clearRect(0, 0, cssWidth, cssHeight);
  ctx.imageSmoothingEnabled = false;

  const faces = buildFaces(image);
  const origin = { x: 0, y: 0 };
  const baseScale = 1;
  const projectedFaces = faces.map((face) => ({
    ...face,
    points2d: face.points3d.map((point) => project(point, baseScale, origin)) as [
      Vec2,
      Vec2,
      Vec2,
      Vec2,
    ],
    depth:
      face.points3d.reduce((sum, point) => sum + point.x + point.z + point.y * 0.72, 0) /
      face.points3d.length,
  }));

  const bounds = quadBounds(projectedFaces.flatMap((face) => face.points2d));
  const paddingX = cssWidth * 0.15;
  const paddingY = cssHeight * 0.1;
  const scale = Math.min(
    (cssWidth - paddingX * 2) / Math.max(bounds.maxX - bounds.minX, 1),
    (cssHeight - paddingY * 2) / Math.max(bounds.maxY - bounds.minY, 1),
  );

  const scaledWidth = (bounds.maxX - bounds.minX) * scale;
  const scaledHeight = (bounds.maxY - bounds.minY) * scale;
  const centeredOrigin = {
    x: paddingX + (cssWidth - paddingX * 2 - scaledWidth) / 2 - bounds.minX * scale,
    y: paddingY + (cssHeight - paddingY * 2 - scaledHeight) / 2 - bounds.minY * scale,
  };

  projectedFaces
    .map((face) => ({
      ...face,
      points2d: face.points3d.map((point) => project(point, scale, centeredOrigin)) as [
        Vec2,
        Vec2,
        Vec2,
        Vec2,
      ],
    }))
    .sort((left, right) => left.depth - right.depth)
    .forEach((face) => {
      drawTexturedQuad(
        ctx,
        { data: texture.data, width: texture.width, height: texture.height },
        face.region,
        face.points2d,
        face.shade,
      );
    });
}

export default function SkinPreview2D({ src, alt, className, onError }: Props) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    let cancelled = false;
    let resizeFrame = 0;

    const render = async () => {
      try {
        const image = await loadImage(src);
        if (cancelled || !canvasRef.current) return;
        renderSkinPreview(canvasRef.current, image);
      } catch {
        if (!cancelled) onError?.();
      }
    };

    const observer = new ResizeObserver(() => {
      window.cancelAnimationFrame(resizeFrame);
      resizeFrame = window.requestAnimationFrame(() => {
        void render();
      });
    });

    observer.observe(canvas);
    void render();

    return () => {
      cancelled = true;
      window.cancelAnimationFrame(resizeFrame);
      observer.disconnect();
    };
  }, [onError, src]);

  return <canvas ref={canvasRef} className={className} aria-label={alt} role="img" />;
}
