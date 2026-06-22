export const minCockpitOpacity = 35;
export const maxCockpitOpacity = 100;

export interface CockpitTransparency {
  shellBackground: string;
  headerBackground: string;
  panelBackground: string;
  panelSoftBackground: string;
  controlBackground: string;
  progressTrack: string;
  glowBackground: string;
}

export function getCockpitTransparency(opacity: number): CockpitTransparency {
  const value = clamp(opacity, minCockpitOpacity, maxCockpitOpacity);
  const ratio = (value - minCockpitOpacity) / (maxCockpitOpacity - minCockpitOpacity);

  return {
    shellBackground: rgba(246, 248, 252, lerp(0.38, 0.95, ratio)),
    headerBackground: rgba(255, 255, 255, lerp(0.3, 0.82, ratio)),
    panelBackground: rgba(255, 255, 255, lerp(0.46, 0.92, ratio)),
    panelSoftBackground: rgba(255, 255, 255, lerp(0.34, 0.76, ratio)),
    controlBackground: rgba(255, 255, 255, lerp(0.5, 0.96, ratio)),
    progressTrack: rgba(230, 237, 245, lerp(0.5, 0.92, ratio)),
    glowBackground: [
      `radial-gradient(circle at 8% 0%, ${rgba(218, 226, 242, lerp(0.2, 0.56, ratio))}, transparent 36%)`,
      `radial-gradient(circle at 96% 8%, ${rgba(214, 235, 229, lerp(0.16, 0.42, ratio))}, transparent 34%)`,
    ].join(', '),
  };
}

function clamp(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return max;
  return Math.min(max, Math.max(min, value));
}

function lerp(start: number, end: number, ratio: number): number {
  return start + (end - start) * ratio;
}

function rgba(red: number, green: number, blue: number, alpha: number): string {
  return `rgba(${red}, ${green}, ${blue}, ${roundAlpha(alpha)})`;
}

function roundAlpha(alpha: number): number {
  return Math.round(alpha * 100) / 100;
}
