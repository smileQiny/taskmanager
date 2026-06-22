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
    shellBackground: rgba(33, 37, 39, lerp(0.5, 0.92, ratio)),
    headerBackground: rgba(14, 18, 21, lerp(0.48, 0.82, ratio)),
    panelBackground: rgba(5, 9, 12, lerp(0.62, 0.92, ratio)),
    panelSoftBackground: rgba(18, 22, 25, lerp(0.5, 0.78, ratio)),
    controlBackground: rgba(5, 9, 12, lerp(0.74, 0.96, ratio)),
    progressTrack: rgba(72, 78, 82, lerp(0.42, 0.78, ratio)),
    glowBackground: [
      `linear-gradient(180deg, ${rgba(255, 255, 255, lerp(0.08, 0.14, ratio))}, transparent 42%)`,
      `repeating-linear-gradient(0deg, ${rgba(255, 255, 255, lerp(0.025, 0.045, ratio))} 0px, ${rgba(255, 255, 255, lerp(0.025, 0.045, ratio))} 1px, transparent 1px, transparent 4px)`,
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
