import { describe, expect, it } from 'vitest';
import { getCockpitTransparency } from './cockpitTransparency';

describe('cockpit transparency', () => {
  it('maps low slider values to see-through window surfaces', () => {
    const transparent = getCockpitTransparency(35);

    expect(transparent.shellBackground).toBe('rgba(38, 44, 49, 0.54)');
    expect(transparent.panelBackground).toBe('rgba(19, 25, 30, 0.5)');
    expect(transparent.headerBackground).toBe('rgba(25, 31, 36, 0.5)');
  });

  it('keeps full opacity crisp without using element opacity', () => {
    const solid = getCockpitTransparency(100);

    expect(solid.shellBackground).toBe('rgba(38, 44, 49, 0.88)');
    expect(solid.panelBackground).toBe('rgba(19, 25, 30, 0.82)');
    expect(solid.headerBackground).toBe('rgba(25, 31, 36, 0.78)');
  });

  it('clamps out-of-range values', () => {
    expect(getCockpitTransparency(10).shellBackground).toBe(getCockpitTransparency(35).shellBackground);
    expect(getCockpitTransparency(130).shellBackground).toBe(getCockpitTransparency(100).shellBackground);
  });
});
