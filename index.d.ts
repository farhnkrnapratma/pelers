export type PelersErrorCode = 'unsupported-diagram' | 'syntax' | 'semantic' | 'limit';

export declare class PelersError extends Error {
    readonly code: PelersErrorCode;
    constructor(message: string, code: PelersErrorCode);
}

export interface RenderResult {
    svg: string;
    width: number;
    height: number;
    title?: string;
}

export declare function render(source: string): string;
export declare function renderFull(source: string): RenderResult;
export declare function detectType(source: string): string | null;
export declare function supports(source: string): boolean;
