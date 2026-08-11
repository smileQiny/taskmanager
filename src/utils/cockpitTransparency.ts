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
    shellBackground: rgba(38, 44, 49, lerp(0.54, 0.88, ratio)),
    headerBackground: rgba(25, 31, 36, lerp(0.5, 0.78, ratio)),
    panelBackground: rgba(19, 25, 30, lerp(0.5, 0.82, ratio)),
    panelSoftBackground: rgba(31, 37, 42, lerp(0.42, 0.68, ratio)),
    controlBackground: rgba(22, 28, 33, lerp(0.52, 0.82, ratio)),
    progressTrack: rgba(91, 101, 110, lerp(0.3, 0.58, ratio)),
    glowBackground: [
      `linear-gradient(180deg, ${rgba(255, 255, 255, lerp(0.06, 0.1, ratio))}, transparent 38%)`,
      `radial-gradient(circle at 18% 8%, ${rgba(156, 185, 205, lerp(0.08, 0.14, ratio))}, transparent 42%)`,
      `radial-gradient(circle at 92% 94%, ${rgba(97, 119, 133, lerp(0.08, 0.13, ratio))}, transparent 46%)`,
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
