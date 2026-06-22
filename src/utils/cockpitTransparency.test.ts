import { describe, expect, it } from 'vitest';
import { getCockpitTransparency } from './cockpitTransparency';

describe('cockpit transparency', () => {
  it('maps low slider values to see-through window surfaces', () => {
    const transparent = getCockpitTransparency(35);

    expect(transparent.shellBackground).toBe('rgba(33, 37, 39, 0.5)');
    expect(transparent.panelBackground).toBe('rgba(5, 9, 12, 0.62)');
    expect(transparent.headerBackground).toBe('rgba(14, 18, 21, 0.48)');
  });

  it('keeps full opacity crisp without using element opacity', () => {
    const solid = getCockpitTransparency(100);

    expect(solid.shellBackground).toBe('rgba(33, 37, 39, 0.92)');
    expect(solid.panelBackground).toBe('rgba(5, 9, 12, 0.92)');
    expect(solid.headerBackground).toBe('rgba(14, 18, 21, 0.82)');
  });

  it('clamps out-of-range values', () => {
    expect(getCockpitTransparency(10).shellBackground).toBe(getCockpitTransparency(35).shellBackground);
    expect(getCockpitTransparency(130).shellBackground).toBe(getCockpitTransparency(100).shellBackground);
  });
});
