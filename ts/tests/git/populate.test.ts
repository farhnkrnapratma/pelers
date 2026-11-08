// Ported by hand from the tests written inside Mermaid 12.1.0's
// packages/mermaid/src/diagrams/git/gitGraphParser.ts, which have no spec file of their own.
// Copyright (c) 2014 - 2022 Knut Sveidqvist, MIT License.
import { describe, expect, it, vi } from 'vitest';
import { parseStatement, populate, type GitDbParseProvider } from '../../src/diagrams/git/db.js';
import type { GitGraphAst } from '../../src/diagrams/git/parser.js';

const mockDB: GitDbParseProvider = {
  setDirection: vi.fn(),
  commit: vi.fn(),
  branch: vi.fn(),
  merge: vi.fn(),
  cherryPick: vi.fn(),
  checkout: vi.fn(),
};

describe('GitGraph Parser', () => {
  it('should parse a commit statement', () => {
    parseStatement({ $type: 'Commit', id: '1', message: 'test', tags: ['tag1', 'tag2'], type: 'NORMAL' }, mockDB);
    expect(mockDB.commit).toHaveBeenCalledWith({ id: '1', msg: 'test', tags: ['tag1', 'tag2'], type: 0 });
  });

  it('should parse a branch statement', () => {
    parseStatement({ $type: 'Branch', name: 'newBranch', order: 1 }, mockDB);
    expect(mockDB.branch).toHaveBeenCalledWith({ name: 'newBranch', order: 1 });
  });

  it('should parse a checkout statement', () => {
    parseStatement({ $type: 'Checkout', branch: 'newBranch' }, mockDB);
    expect(mockDB.checkout).toHaveBeenCalledWith('newBranch');
  });

  it('should parse a merge statement', () => {
    parseStatement({ $type: 'Merge', branch: 'newBranch', id: '1', tags: ['tag1', 'tag2'], type: 'NORMAL' }, mockDB);
    expect(mockDB.merge).toHaveBeenCalledWith({ branch: 'newBranch', id: '1', tags: ['tag1', 'tag2'], type: 0 });
  });

  it('should parse a cherry picking statement', () => {
    parseStatement({ $type: 'CherryPicking', id: '1', tags: ['tag1', 'tag2'], parent: '2' }, mockDB);
    expect(mockDB.cherryPick).toHaveBeenCalledWith({ id: '1', targetId: '', parent: '2', tags: ['tag1', 'tag2'] });
  });

  it('should parse a langium generated gitGraph ast', () => {
    const ast: GitGraphAst = {
      $type: 'GitGraph',
      statements: [
        { $type: 'Commit', id: '1', message: 'test', tags: ['tag1', 'tag2'], type: 'NORMAL' },
        { $type: 'Branch', name: 'newBranch', order: 1 },
        { $type: 'Merge', branch: 'newBranch', id: '1', tags: ['tag1', 'tag2'], type: 'NORMAL' },
        { $type: 'Checkout', branch: 'newBranch' },
        { $type: 'CherryPicking', id: '1', tags: ['tag1', 'tag2'], parent: '2' },
      ],
      accDescr: '',
      accTitle: '',
      title: '',
    };

    populate(ast, mockDB);

    expect(mockDB.commit).toHaveBeenCalledWith({ id: '1', msg: 'test', tags: ['tag1', 'tag2'], type: 0 });
    expect(mockDB.branch).toHaveBeenCalledWith({ name: 'newBranch', order: 1 });
    expect(mockDB.merge).toHaveBeenCalledWith({ branch: 'newBranch', id: '1', tags: ['tag1', 'tag2'], type: 0 });
    expect(mockDB.checkout).toHaveBeenCalledWith('newBranch');
  });
});
