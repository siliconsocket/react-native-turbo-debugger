import type {JsonValue, StateMetadata} from './index';

export function attachZustand<State>(
  store: {subscribe(listener: (after: State, before: State) => void): () => void},
  options: {
    name: string;
    project: (state: State) => JsonValue;
    describe?: (after: State, before: State) => StateMetadata | undefined;
  },
): () => void;
