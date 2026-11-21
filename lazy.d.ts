import type {
  DiagramType,
  MountOptions,
  Mounted,
  RenderOptions,
  RenderResult,
} from "./index.js";

export * from "./index.js";

export declare function load(type?: DiagramType | null): Promise<boolean>;
export declare function renderAsync(
  text: string,
  options?: RenderOptions,
): Promise<RenderResult>;
export declare function mountAsync(
  element: HTMLElement,
  text: string,
  options?: MountOptions,
): Promise<Mounted>;
