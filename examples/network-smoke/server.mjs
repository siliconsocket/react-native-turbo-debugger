import {createServer} from 'node:http';

const paths = new Map([
  ['GET /smoke-fetch', 200],
  ['POST /smoke-xhr', 503],
]);

createServer((request, response) => {
  const path = new URL(request.url, 'http://127.0.0.1').pathname;
  const status = paths.get(`${request.method} ${path}`) ?? 404;
  request.resume();
  request.once('end', () => {
    console.log(request.method, path, status);
    response.writeHead(status, {'Content-Type': 'application/json'});
    response.end(JSON.stringify({status}));
  });
}).listen(7411, '127.0.0.1');
