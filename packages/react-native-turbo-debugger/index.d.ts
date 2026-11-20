export type JsonValue =
  | null
  | boolean
  | number
  | string
  | JsonValue[]
  | { [key: string]: JsonValue };

export interface StateMetadata {
  action?: string;
  payload?: JsonValue;
  correlationId?: string;
}

export interface StateTransition extends StateMetadata {
  source: string;
  store: string;
  action: string;
  before: JsonValue;
  after: JsonValue;
}

export function sendState(transition: StateTransition): void;
