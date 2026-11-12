import { describe, expect, it } from 'vitest';
import { PelersError, parse, render, supports } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { elements } from '../support/xml.js';

const options = { measurer: metricsMeasurer };

const FLOW = `
  commit id: "init"
  commit type: REVERSE tag: "v0.1"
  branch develop
  commit id: "feat" msg: "A message"
  commit type: HIGHLIGHT
  checkout main
  commit tag: "v0.2" tag: "stable"
  merge develop id: "merged" tag: "v1.0"
  branch hotfix
  commit id: "fix"
  checkout develop
  commit
  cherry-pick id: "fix"
  checkout main
  merge hotfix
`;

const withConfig = (yaml: string, body: string): string => `---\nconfig:\n  gitGraph:\n${yaml}\n---\n${body}`;
const count = (svg: string, text: string): number => svg.split(text).length - 1;

// Centers of the commit symbols, in drawing order, which is the order the commits were made in.
function commitCenters(svg: string): { x: number; y: number }[] {
  const out: { x: number; y: number }[] = [];
  for (const group of svg.split('<g class="pelers-commit ').slice(1)) {
    const circle = /<circle cx="([-\d.]+)" cy="([-\d.]+)"/.exec(group);
    const rect = /^[^>]*>(?:<title>[^<]*<\/title>)?<rect x="([-\d.]+)" y="([-\d.]+)"/.exec(group);
    if (rect) out.push({ x: Number(rect[1]) + 8, y: Number(rect[2]) + 8 });
    else if (circle) out.push({ x: Number(circle[1]), y: Number(circle[2]) });
  }
  return out;
}

describe('git graph rendering', () => {
  it('draws a lane per branch and a symbol per commit', () => {
    const { svg, type } = render('gitGraph\n' + FLOW, options);
    expect(type).toBe('gitGraph');
    expect(supports('gitGraph\n commit')).toBe(true);
    expect(svg).toContain('class="pelers pelers-gitGraph"');
    expect(count(svg, 'class="pelers-branch"')).toBe(3);
    expect(count(svg, 'class="pelers-lane"')).toBe(3);
    expect(count(svg, '<g class="pelers-commit ')).toBe(10);
    expect(svg).toContain('data-id="develop"');
    expect(svg).toContain('>hotfix<');
    for (const k of [1, 2, 3]) expect(svg).toContain(`var(--pelers-series-${k},`);
  });

  it('gives each commit type its own symbol', () => {
    const { svg } = render('gitGraph\n' + FLOW, options);
    expect(count(svg, 'pelers-commit-normal"')).toBe(5);
    expect(count(svg, 'pelers-commit-reverse"')).toBe(1);
    expect(count(svg, 'pelers-commit-highlight"')).toBe(1);
    expect(count(svg, 'pelers-commit-merge"')).toBe(2);
    expect(count(svg, 'pelers-commit-cherry-pick"')).toBe(1);
    const highlight = svg.split('pelers-commit-highlight"')[1].split('</g>')[0];
    expect(highlight).toContain('<rect');
    const merge = svg.split('pelers-commit-merge"')[1].split('</g>')[0];
    expect(count(merge, '<circle')).toBe(2);
  });

  it('draws a merge with a type as that type, and still marks it as a merge', () => {
    const { svg } = render(
      'gitGraph\n commit\n branch b\n commit\n checkout main\n commit\n merge b type: REVERSE',
      options
    );
    expect(svg).toContain('class="pelers-commit pelers-commit-reverse pelers-commit-merge"');
  });

  it('labels commits with their ids, except unnamed merges and cherry-picks', () => {
    const { svg } = render('gitGraph\n' + FLOW, options);
    expect(svg).toContain('>init<');
    expect(svg).toContain('>merged<');
    expect(count(svg, 'class="pelers-commit-label"')).toBe(8);
    expect(count(svg, 'rotate(-45 ')).toBe(8);
    const hidden = render(withConfig('    showCommitLabel: false', 'gitGraph\n' + FLOW), options).svg;
    expect(hidden).not.toContain('pelers-commit-label');
    expect(hidden).toContain('>v0.1<');
  });

  it('keeps ids level when rotateCommitLabel is off, and spaces commits to fit them', () => {
    const src = 'gitGraph\n commit id: "a rather long commit id"\n commit id: "another long commit id"\n commit';
    const rotated = render(src, options);
    const level = render(withConfig('    rotateCommitLabel: false', src), options);
    expect(level.svg).not.toContain('rotate(');
    expect(level.width).toBeGreaterThan(rotated.width);
    expect(level.height).toBeLessThan(rotated.height);
  });

  it('draws tags, the default cherry-pick tag, and no empty tag', () => {
    const { svg } = render('gitGraph\n' + FLOW, options);
    expect(count(svg, 'class="pelers-tag"')).toBe(5);
    expect(svg).toContain('>cherry-pick:fix<');
    const none = render(
      'gitGraph\n commit id:"a" tag:""\n branch b\n commit id:"c"\n checkout main\n commit\n cherry-pick id:"c" tag:""',
      options
    ).svg;
    expect(none).not.toContain('pelers-tag');
  });

  it('connects commits: straight along a lane, curved between lanes, dashed for a cherry-pick', () => {
    const { svg } = render('gitGraph\n' + FLOW, options);
    expect(count(svg, 'class="pelers-edge"')).toBe(9);
    expect(count(svg, 'class="pelers-edge pelers-edge-merge"')).toBe(2);
    expect(count(svg, 'class="pelers-edge pelers-edge-cherry-pick"')).toBe(1);
    const cherry = svg.split('pelers-edge-cherry-pick"')[1].split('/>')[0];
    expect(cherry).toContain('stroke-dasharray');
    const merge = svg.split('pelers-edge-merge"')[1].split('/>')[0];
    expect(merge).toMatch(/d="M[^"]*C[^"]*"/);
    expect(merge).toContain('var(--pelers-series-2,');
  });

  it('writes ids, tags, and branch names as plain text, with entity codes decoded and <br> as a line break', () => {
    const { svg } = render(
      'gitGraph\n commit id:"add <Button> #35;1" tag:"<i>t</i>"\n branch "one<br>two"\n commit id:"**a**"',
      options
    );
    expect(svg).toContain('>add &lt;Button&gt; #1<');
    expect(svg).toContain('>&lt;i&gt;t&lt;/i&gt;<');
    expect(svg).toContain('>**a**<');
    expect(svg).toMatch(/<tspan[^>]*>one<\/tspan><tspan[^>]*>two<\/tspan>/);
    expect(svg).toContain('data-id="add &lt;Button&gt; #1"');
  });

  it('puts the commit message in a tooltip', () => {
    expect(render('gitGraph\n commit id:"a" msg:"Fix <the> bug"', options).svg).toContain(
      '<title>Fix &lt;the&gt; bug</title>'
    );
  });

  it('lays lanes out across the page for TB and upside down for BT', () => {
    const body = '\n commit\n commit\n branch b\n commit\n commit\n checkout main\n commit';
    const lr = render('gitGraph' + body, options);
    const tb = render('gitGraph TB:' + body, options);
    const bt = render('gitGraph BT:' + body, options);
    const [l, t, b] = [lr, tb, bt].map((r) => commitCenters(r.svg));
    expect(l).toHaveLength(5);
    expect(l[1].x).toBeGreaterThan(l[0].x);
    expect(l[1].y).toBe(l[0].y);
    expect(l[2].y).toBeGreaterThan(l[0].y);
    expect(t[1].y).toBeGreaterThan(t[0].y);
    expect(t[1].x).toBe(t[0].x);
    expect(t[2].x).toBeGreaterThan(t[0].x);
    expect(b[1].y).toBeLessThan(b[0].y);
    expect(b[2].x).toBeGreaterThan(b[0].x);
    expect(tb.height).toBeGreaterThan(tb.width);
    expect(bt.height).toBe(tb.height);
    expect(bt.svg).toContain('rotate(45 ');
  });

  it('shares positions between branches with parallelCommits', () => {
    const src = 'gitGraph\n commit\n branch develop\n commit\n commit\n checkout main\n commit\n commit';
    const serial = render(src, options);
    const parallel = render(withConfig('    parallelCommits: true', src), options);
    expect(parallel.width).toBeLessThan(serial.width);
    const centers = commitCenters(parallel.svg);
    expect(centers[3].x).toBe(centers[1].x);
    expect(centers[4].x).toBe(centers[2].x);
  });

  it('hides branch names and lane lines when showBranches is off', () => {
    const { svg } = render(withConfig('    showBranches: false', 'gitGraph\n' + FLOW), options);
    expect(svg).not.toContain('pelers-branch');
    expect(svg).not.toContain('>develop<');
    expect(count(svg, '<g class="pelers-commit ')).toBe(10);
  });

  it('names and orders the main branch from config, and orders branches by `order`', () => {
    const src = 'gitGraph\n commit\n branch b1\n commit\n branch b2 order: 3\n commit\n branch b3 order: 1\n commit';
    const model = parse(withConfig('    mainBranchName: trunk\n    mainBranchOrder: 2', src));
    expect(model.type).toBe('gitGraph');
    if (model.type !== 'gitGraph') return;
    expect(model.lanes).toEqual(['b1', 'b3', 'trunk', 'b2']);
    const { svg } = render(withConfig('    mainBranchName: trunk\n    mainBranchOrder: 2', src), options);
    expect(svg.indexOf('data-id="b3"')).toBeLessThan(svg.indexOf('data-id="trunk"'));
    expect(render('%%{init: {"gitGraph": {"mainBranchName": "dev"}}}%%\ngitGraph\n commit', options).svg).toContain(
      '>dev<'
    );
  });

  it('reads branch names that start with title', () => {
    const model = parse(
      'gitGraph\n commit\n branch titles\n commit\n branch title/x\n commit\n checkout main\n merge titles\n branch title-2.0\n'
    );
    expect(model.type).toBe('gitGraph');
    if (model.type !== 'gitGraph') return;
    expect(model.lanes).toEqual(['main', 'titles', 'title/x', 'title-2.0']);
    expect(model.title).toBeUndefined();
    expect(() => parse('gitGraph\n commit\n branch title.v2\n checkout title.v2')).not.toThrow();
  });

  it('takes the title from front matter unless the graph sets one', () => {
    expect(render('---\ntitle: From front matter\n---\ngitGraph\n commit', options).svg).toContain('From front matter');
    const own = render('---\ntitle: From front matter\n---\ngitGraph\n title Own\n commit', options).svg;
    expect(own).toContain('>Own<');
    expect(own).not.toContain('From front matter');
  });

  it('writes the accessible title and description', () => {
    const { svg } = render('gitGraph\n accTitle: History\n accDescr: Two branches\n commit', options);
    expect(svg).toContain('<title id="pelers-title">History</title>');
    expect(svg).toContain('<desc id="pelers-desc">Two branches</desc>');
  });

  it("generates the same commit ids every time, in Mermaid's shape", () => {
    const src = 'gitGraph\n commit\n commit\n branch b\n commit\n checkout main\n commit\n merge b';
    const model = parse(src);
    if (model.type !== 'gitGraph') throw new Error('not a git graph');
    const ids = [...model.commits.keys()];
    expect(ids).toHaveLength(5);
    ids.forEach((id, seq) => expect(id).toMatch(new RegExp(`^${seq}-[0-9a-f]{7}$`)));
    expect(new Set(ids).size).toBe(5);
    expect(render(src, options).svg).toBe(render(src, options).svg);
    const again = parse(src);
    expect(again.type === 'gitGraph' && [...again.commits.keys()]).toEqual(ids);
  });

  it('does not let a generated id replace a commit that already has it', () => {
    const first = parse('gitGraph\n commit');
    if (first.type !== 'gitGraph') throw new Error('not a git graph');
    const taken = [...first.commits.keys()][0].replace(/^0/, '1');
    const model = parse(`gitGraph\n commit id:"${taken}"\n commit`);
    if (model.type !== 'gitGraph') throw new Error('not a git graph');
    expect(model.commits.size).toBe(2);
    expect(model.warnings).toEqual([]);
  });

  it("keeps Mermaid's rule for a repeated commit id: the later commit replaces the earlier, with a warning", () => {
    const model = parse('gitGraph\n commit id:"a"\n commit id:"b"\n commit id:"a"');
    if (model.type !== 'gitGraph') throw new Error('not a git graph');
    expect([...model.commits.keys()]).toEqual(['a', 'b']);
    expect(model.commits.get('a')?.seq).toBe(2);
    expect(model.warnings).toEqual(['Commit ID a already exists']);
    expect(render('gitGraph\n commit id:"a"\n commit id:"a"\n branch b\n commit id:"a"', options).svg).toContain(
      'pelers-commit'
    );
  });

  it('rejects what Mermaid rejects, with its messages', () => {
    const cases: [string, string][] = [
      [
        'gitGraph\n commit\n branch main',
        'Trying to create an existing branch. (Help: Either use a new name if you want create a new branch or try using "checkout main")',
      ],
      [
        'gitGraph\n commit\n checkout nope',
        'Trying to checkout branch which is not yet created. (Help try using "branch nope")',
      ],
      ['gitGraph\n commit\n branch b\n merge b', 'Incorrect usage of "merge". Cannot merge a branch to itself'],
      ['gitGraph\n commit\n branch b\n merge main', "Cannot merge branch 'main' into itself."],
      ['gitGraph\n merge b', 'Incorrect usage of "merge". Current branch (main)has no commits'],
      ['gitGraph\n commit\n merge b', 'Incorrect usage of "merge". Branch to be merged (b) does not exist'],
      [
        'gitGraph\n branch b\n checkout main\n commit\n merge b',
        'Incorrect usage of "merge". Branch to be merged (b) has no commits',
      ],
      [
        'gitGraph\n commit\n branch b\n checkout main\n merge b',
        'Incorrect usage of "merge". Both branches have same head',
      ],
      [
        'gitGraph\n commit id:"a"\n branch b\n commit\n checkout main\n commit\n merge b id:"a"',
        'Incorrect usage of "merge". Commit with id:a already exists, use different custom id',
      ],
      [
        'gitGraph\n commit\n cherry-pick id:"nope"',
        'Incorrect usage of "cherryPick". Source commit id should exist and provided',
      ],
      [
        'gitGraph\n commit\n cherry-pick',
        'Incorrect usage of "cherryPick". Source commit id should exist and provided',
      ],
      [
        'gitGraph\n commit id:"a"\n cherry-pick id:"a"',
        'Incorrect usage of "cherryPick". Source commit is already on current branch',
      ],
      [
        'gitGraph\n branch b\n commit id:"a"\n checkout main\n cherry-pick id:"a"',
        'Incorrect usage of "cherry-pick". Current branch (main)has no commits',
      ],
      [
        'gitGraph\n commit id:"z"\n branch b\n commit id:"a"\n checkout main\n commit\n cherry-pick id:"a" parent:"q"',
        'Invalid operation: The specified parent commit is not an immediate parent of the cherry-picked commit.',
      ],
      [
        'gitGraph\n commit\n branch b\n commit\n checkout main\n commit\n merge b id:"m"\n checkout b\n cherry-pick id:"m"',
        'Incorrect usage of cherry-pick: If the source commit is a merge commit, an immediate parent commit must be specified.',
      ],
    ];
    for (const [src, message] of cases) {
      let error: unknown;
      try {
        render(src, options);
      } catch (e) {
        error = e;
      }
      expect(error, src).toBeInstanceOf(PelersError);
      expect((error as PelersError).message, src).toBe(message);
      expect((error as PelersError).code, src).toBe('semantic');
    }
  });

  it('draws a graph with no commits, and one with only branches', () => {
    expect(render('gitGraph', options).svg).toContain('>main<');
    const { svg } = render('gitGraph\n branch a\n branch "b c"', options);
    expect(count(svg, 'class="pelers-branch"')).toBe(3);
    expect(svg).toContain('>b c<');
    expect(svg).not.toContain('NaN');
  });

  it('emits only finite coordinates', () => {
    for (const header of ['gitGraph', 'gitGraph TB:', 'gitGraph BT:']) {
      for (const config of ['    parallelCommits: true', '    rotateCommitLabel: false', '    showBranches: false']) {
        const { svg, width, height } = render(withConfig(config, header + '\n' + FLOW), options);
        expect(Number.isFinite(width) && Number.isFinite(height)).toBe(true);
        expect(svg).not.toMatch(/NaN|Infinity|undefined/);
        for (const el of elements(svg)) {
          for (const [name, value] of el.attrs) {
            if (name === 'cx' || name === 'cy' || name === 'x' || name === 'y')
              expect(Number.isFinite(Number(value)), name).toBe(true);
          }
        }
      }
    }
  });
});
