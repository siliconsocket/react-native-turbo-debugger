import React, {useEffect, useState} from 'react';
import {Text, View} from 'react-native';

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

export default function App() {
  const [result, setResult] = useState('Running network smoke test');

  useEffect(() => {
    (async () => {
      const ok = await fetch(base + '/smoke-fetch');
      const failed = await xhr();
      const result = `Network smoke: fetch=${ok.status} xhr=${failed}`;
      console.log(result);
      setResult(result);
    })().catch(error => {
      console.error(error);
      setResult(String(error));
    });
  }, []);

  return (
    <View style={{flex: 1, alignItems: 'center', justifyContent: 'center'}}>
      <Text accessibilityLabel={result}>{result}</Text>
    </View>
  );
}
