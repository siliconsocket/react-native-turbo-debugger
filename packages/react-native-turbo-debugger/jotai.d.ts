import type {Atom, createStore} from 'jotai/vanilla';
import type {JsonValue} from './index';

export function attachJotaiAtom<Value>(
  store: ReturnType<typeof createStore>,
  atom: Atom<Value>,
  options: {name: string; action?: string; project: (value: Awaited<Value>) => JsonValue},
): () => void;
