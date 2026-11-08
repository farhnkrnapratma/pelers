import { describe, expect, it } from 'vitest';
import { LEAD, routePath } from '../../src/svg/edges.js';

// The numbers of the one curve in a path between two points: the two handles and where it ends.
// The path may run straight for a little way before and after it.
function curve(...args: Parameters<typeof routePath>): number[] {
  const { d } = routePath(...args);
  expect(d).toMatch(/^M[-\d.,]+(L[-\d.,]+)?C[-\d., ]+(L[-\d.,]+)?$/);
  return d
    .slice(d.indexOf('C') + 1)
    .split('L')[0]
    .split(/[ ,]/)
    .map(Number);
}

describe('edge paths', () => {
  it('draws one curve between two points that are not in line, leaving and arriving along the flow', () => {
    for (const [across, along] of [
      [100, 48],
      [-100, 48],
      [100, 200],
      [30, 48],
    ]) {
      const down = routePath([0, 0, 0, across, along, 0], undefined, 0, 0);
      expect(down.d).toMatch(/^M0,0C[-\d., ]+$/);
      // Each handle is in line with its end, so the curve leaves and arrives straight down.
      const [x1, y1, x2, y2, x, y] = curve([0, 0, 0, across, along, 0], undefined, 0, 0);
      expect([x1, x2, x, y]).toEqual([0, across, across, along]);
      expect(y1).toBeGreaterThan(0);
      expect(y2).toBeLessThan(along);
      expect([down.sdx + 0, down.sdy, down.edx + 0, down.edy]).toEqual([0, -1, 0, 1]);
      // And the same on its side, in a diagram that runs from left to right.
      const [sx1, sy1, sx2, sy2] = curve([0, 0, 1, along, across, 1], undefined, 0, 0);
      expect([sx1, sy1, sx2, sy2]).toEqual([y1, 0, y2, across]);
    }
  });

  it('turns sooner on a run much wider than the gap it crosses, and never sharper than a limit', () => {
    // Up to eight times as far across as along, the handles reach half of the way: an even S.
    const [, even1, , even2] = curve([0, 0, 0, 200, 48, 0], undefined, 0, 0);
    expect(even1).toBeCloseTo(24, 5);
    expect(even2).toBeCloseTo(24, 5);
    // 480 across is ten times as far, and 4800 no sharper than sixteen times.
    const [, y1, , y2] = curve([0, 0, 0, 480, 48, 0], undefined, 0, 0);
    expect(y1).toBeCloseTo(19.2, 5);
    expect(y2).toBeCloseTo(28.8, 5);
    const [, far1, , far2] = curve([0, 0, 0, -4800, 48, 0], undefined, 0, 0);
    expect(far1).toBeCloseTo(12, 5);
    expect(far2).toBeCloseTo(36, 5);
    const [x1, , x2] = curve([0, 0, 1, 40, 400, 1], undefined, 0, 0);
    expect(x1).toBeCloseTo(16, 5);
    expect(x2).toBeCloseTo(24, 5);
  });

  it('runs straight into a marker, and turns as far from a node whatever marker its end carries', () => {
    const route = [0, 0, 0, 40, 48, 0];
    // The curve stops short of the arrowhead, which takes seven, and a straight run joins them.
    const arrow = routePath(route, undefined, 0, 7);
    expect(arrow.d).toMatch(/C[-\d., ]+ 40,35L40,41$/);
    expect(curve(route, undefined, 0, 7).slice(4)).toEqual([40, 48 - 7 - LEAD]);
    const leaving = routePath(route, undefined, 7, 0);
    expect(leaving.d).toMatch(/^M0,7L0,13C/);
    // An end without a marker turns where its neighbours on the same side of the node do.
    expect(curve(route, undefined, 0, 0, 0, 7 + LEAD).slice(4)).toEqual([40, 35]);
    expect(curve(route, undefined, 0, 8, 0, 8 + LEAD).slice(4)).toEqual(
      curve(route, undefined, 0, 7, 0, 8 + LEAD).slice(4)
    );
    // With little room, the straight run takes no more than a quarter of the way.
    expect(curve([0, 0, 0, 15, 19, 0], undefined, 0, 7).slice(4)).toEqual([15, 9]);
    // A straight edge has nothing to run clear of.
    expect(routePath([0, 0, 0, 0, 48, 0], undefined, 0, 7).d).toBe('M0,0L0,41');
  });

  it('keeps both ends of a curve heading along the flow, however far across it runs', () => {
    for (const across of [40, 96, -96, 900]) {
      const path = routePath([0, 0, 0, across, 48, 0], undefined, 0, 7);
      expect([path.sdx + 0, path.sdy]).toEqual([0, -1]);
      expect([path.edx + 0, path.edy]).toEqual([0, 1]);
    }
  });

  it('turns a corner of the route as widely as its legs allow, rounds it in a stepped route, and leaves it sharp in a linear one', () => {
    const route = [0, 0, 0, 0, 20, 0, 100, 20, 0, 100, 48, 0];
    // Each turn takes the whole of the leg that ends at a node and half of the one they share.
    expect(routePath(route, undefined, 0, 0).d).toMatch(/^M0,0C[-\d., ]+ 50,20C[-\d., ]+ 100,48$/);
    expect(routePath(route, 'step', 0, 0).d).toBe('M0,0L0,8Q0,20 12,20L88,20Q100,20 100,32L100,48');
    // The run in the middle is 10 long, so each of its corners takes 5.
    expect(routePath([0, 0, 0, 0, 20, 0, 10, 20, 0, 10, 48, 0], 'step', 0, 0).d).toBe(
      'M0,0L0,15Q0,20 5,20Q10,20 10,25L10,48'
    );
    expect(routePath(route, 'linear', 0, 0).d).toBe('M0,0L0,20L100,20L100,48');
    expect(routePath([0, 0, 0, 0, 20, 0, 0, 48, 0], undefined, 0, 0).d).toBe('M0,0L0,48');
  });

  it('turns a stepped route at right angles, with arrowheads along its last run', () => {
    const step = routePath([0, 0, 0, 100, 48, 0], 'step', 0, 7);
    expect(step.d).toBe('M0,0L0,12Q0,24 12,24L88,24Q100,24 100,36L100,41');
    expect([step.edx, step.edy]).toEqual([0, 1]);
    expect(routePath([0, 0, 0, 100, 48, 0], 'stepAfter', 0, 0).d).toBe('M0,0L0,36Q0,48 12,48L100,48');
    expect(routePath([0, 0, 1, 48, 100, 1], 'stepBefore', 0, 0).d).toBe('M0,0L0,88Q0,100 12,100L48,100');
  });
});
