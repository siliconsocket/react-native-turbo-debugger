import React, {useEffect, useState} from 'react';
import {Button, Text, TurboModuleRegistry, View} from 'react-native';
import type {TurboModule} from 'react-native';

const base = 'http://127.0.0.1:7411';

function xhr(): Promise<number> {
  return new Promise((resolve, reject) => {
    const request = new XMLHttpRequest();
    request.open('POST', base + '/smoke-xhr?marker=omit-this-query');
    request.setRequestHeader('Content-Type', 'text/plain');
    request.onload = () => resolve(request.status);
    request.onerror = () => reject(new Error('XHR failed'));
    request.send('turbo debugger smoke');
  });
}

type Counter = {count: number};

function runZustand() {
  const {createStore} = require('zustand/vanilla') as typeof import('zustand/vanilla');
  const {attachZustand} = require('@siliconsocket/react-native-turbo-debugger/zustand') as
    typeof import('@siliconsocket/react-native-turbo-debugger/zustand');
  const store = createStore<Counter>(() => ({count: 0}));
  const detach = attachZustand(store, {name: 'zustandCounter', project: state => state});
  store.setState({count: 1});
  detach();
}

function runRedux() {
  const {configureStore, createSlice} = require('@reduxjs/toolkit') as typeof import('@reduxjs/toolkit');
  const {createReduxStateMiddleware} = require('@siliconsocket/react-native-turbo-debugger/redux') as
    typeof import('@siliconsocket/react-native-turbo-debugger/redux');
  const counter = createSlice({
    name: 'counter',
    initialState: {count: 10},
    reducers: {increment: state => { state.count += 2; }},
  });
  const store = configureStore({
    reducer: counter.reducer,
    middleware: getDefault => getDefault().concat(
      createReduxStateMiddleware<Counter>({name: 'reduxCounter', project: state => state}),
    ),
  });
  store.dispatch(counter.actions.increment());
}

function runJotai() {
  const {atom, createStore} = require('jotai/vanilla') as typeof import('jotai/vanilla');
  const {attachJotaiAtom} = require('@siliconsocket/react-native-turbo-debugger/jotai') as
    typeof import('@siliconsocket/react-native-turbo-debugger/jotai');
  const store = createStore();
  const counter = atom(20);
  const detach = attachJotaiAtom(store, counter, {name: 'jotaiCounter', project: count => ({count})});
  store.set(counter, 23);
  detach();
}

async function runAsyncJotai() {
  const {atom, createStore} = require('jotai/vanilla') as typeof import('jotai/vanilla');
  const {attachJotaiAtom} = require('@siliconsocket/react-native-turbo-debugger/jotai') as
    typeof import('@siliconsocket/react-native-turbo-debugger/jotai');
  const store = createStore();
  const counter = atom<number | Promise<number>>(30);
  const detach = attachJotaiAtom(store, counter, {name: 'asyncCounter', project: count => ({count})});
  let resolveReplaced!: (count: number) => void;
  store.set(counter, new Promise<number>(resolve => {resolveReplaced = resolve;}));
  store.set(counter, Promise.resolve(34));
  resolveReplaced(99);
  await Promise.resolve();
  store.set(counter, Promise.reject(new Error('expected rejection')));
  await Promise.resolve();
  detach();
}

function runDirect() {
  const {sendState} = require('@siliconsocket/react-native-turbo-debugger') as
    typeof import('@siliconsocket/react-native-turbo-debugger');
  sendState({
    source: 'manual', store: 'directCounter', action: 'Set count',
    before: {count: 40}, after: {count: 45}, payload: {step: 5},
    correlationId: 'direct-smoke',
  });
}

function runDiscard() {
  const {sendState} = require('@siliconsocket/react-native-turbo-debugger') as
    typeof import('@siliconsocket/react-native-turbo-debugger');
  const before = {};
  const transition = {source: 'manual', store: 'discardCounter', action: 'Discard me', before};
  const cyclic: {self?: unknown} = {};
  cyclic.self = cyclic;
  sendState({...transition, after: cyclic as never});
  sendState({...transition, after: '😀'.repeat(5000)});
  const native = TurboModuleRegistry.get<TurboModule & {enqueueState(json: string): void}>('RNTurboDebuggerState');
  native?.enqueueState('{invalid json');
  native?.enqueueState(JSON.stringify({...transition, after: '😀'.repeat(5000)}));
  runDirect();
}

const cases = [
  ['Run Zustand', runZustand],
  ['Run Redux', runRedux],
  ['Run Jotai', runJotai],
  ['Run all three', () => { runZustand(); runRedux(); runJotai(); }],
  ['Run async Jotai', runAsyncJotai],
  ['Run direct capture', runDirect],
  ['Run discard checks', runDiscard],
] as const;

export default function App() {
  const [result, setResult] = useState('Running network smoke test');
  const [scenario, setScenario] = useState('Choose a state scenario');

  useEffect(() => {
    (async () => {
      const ok = await fetch(base + '/smoke-fetch');
      const failed = await xhr();
      const message = `Network smoke: fetch=${ok.status} xhr=${failed}`;
      console.log(message);
      setResult(message);
    })().catch(error => {
      console.error(error);
      setResult(String(error));
    });
  }, []);

  return (
    <View style={{flex: 1, alignItems: 'center', justifyContent: 'center'}}>
      <Text accessibilityLabel={result}>{result}</Text>
      {__DEV__ && (
        <View>
          {cases.map(([label, run]) => (
            <Button key={label} title={label} onPress={async () => {
              await run();
              setScenario(`Completed: ${label}`);
            }} />
          ))}
          <Text>{scenario}</Text>
        </View>
      )}
    </View>
  );
}
