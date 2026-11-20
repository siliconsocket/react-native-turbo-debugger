import type {Middleware, UnknownAction} from '@reduxjs/toolkit';
import type {JsonValue, StateMetadata} from './index';

export function createReduxStateMiddleware<State>(options: {
  name: string;
  project: (state: State) => JsonValue;
  describe?: (action: UnknownAction) => StateMetadata | undefined;
}): Middleware<{}, State>;
