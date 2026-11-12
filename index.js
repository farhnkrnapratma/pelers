import { detectType as nativeDetectType, render as nativeRender, supports } from './index.native.js';

export class PelersError extends Error {
    constructor(message, code) {
        super(message);
        this.name = 'PelersError';
        this.code = code;
    }
}

const fail = error => {
    throw new PelersError(error.message, error.code ?? 'semantic');
};

export function render(source) {
    try {
        return nativeRender(source);
    } catch (error) {
        return fail(error);
    }
}

export function detectType(source) {
    return nativeDetectType(source);
}

export { supports };
