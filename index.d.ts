export type PeleErrorCode = 'unsupported-diagram' | 'syntax' | 'semantic' | 'limit';

export declare class PeleError extends Error {
    readonly code: PeleErrorCode;
    constructor(message: string, code: PeleErrorCode);
}

export interface RenderResult {
    svg: string;
    width: number;
    height: number;
    title?: string;
}

export declare function render(source: string): RenderResult;
export declare function detectType(source: string): string | null;
export declare function supports(source: string): boolean;
