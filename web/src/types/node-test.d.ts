// Minimal ambient declarations so the focused decoding test typechecks
// without adding @types/node; the suite runs on Node's built-in runner
// (`npm test`), which provides these modules at runtime.
declare module 'node:test' {
  export function test(name: string, fn: () => void | Promise<void>): void;
}

declare module 'node:assert/strict' {
  interface AssertStrict {
    doesNotThrow(fn: () => unknown): void;
    throws(fn: () => unknown, expected?: RegExp): void;
    equal(actual: unknown, expected: unknown, message?: string): void;
    notEqual(actual: unknown, expected: unknown, message?: string): void;
    deepEqual(actual: unknown, expected: unknown, message?: string): void;
    ok(value: unknown, message?: string): void;
  }
  const assert: AssertStrict;
  export default assert;
}

declare module 'node:fs' {
  export function readFileSync(path: URL, encoding: 'utf8'): string;
}

declare module 'node:vm' {
  export function runInNewContext(code: string, context: object): unknown;
}
