import type { en } from './locales/en';
type Strings<T> = { [K in keyof T]: T[K] extends string ? string : Strings<T[K]> };
export type Catalog = Strings<typeof en>;
