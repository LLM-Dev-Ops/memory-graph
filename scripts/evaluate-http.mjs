import { evaluateHttp } from '../tests/http-contract.mjs';
console.log(JSON.stringify(await evaluateHttp(process.argv[2] || process.cwd())));
