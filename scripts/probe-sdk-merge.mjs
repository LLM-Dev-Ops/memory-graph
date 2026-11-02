import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {stripTypeScriptTypes} from 'node:module';
const root=resolve(process.argv[2]);
// Execute the actual utility module using Node24's TS transform. Only erased
// type-only imports are normalized; real enum definitions are loaded unchanged.
const uri=s=>'data:text/javascript;base64,'+Buffer.from(s).toString('base64');
const types=stripTypeScriptTypes(readFileSync(resolve(root,'clients/typescript/src/types.ts'),'utf8'),{mode:'transform'});
const source=readFileSync(resolve(root,'clients/typescript/src/utils.ts'),'utf8');
const normalized=source.replace(/import\s*\{[\s\S]*?\}\s*from '\.\/types';/,`import {NodeType,EdgeType} from '${uri(types)}';`);
if(normalized===source)throw Error('Expected import layout changed; adapter must be reviewed');
const {deepMerge}=await import(uri(stripTypeScriptTypes(normalized,{mode:'transform'})));
const results={holdout:[],anchors:[]};
function check(suite,name,fn){try{fn();results[suite].push({name,passed:true});}catch(e){results[suite].push({name,passed:false,error:e.message});}finally{delete Object.prototype.mergeProbe;delete Function.prototype.mergeProbe;}}
check('holdout','JSON __proto__ stays inert data',()=>{
 const target={};deepMerge(target,JSON.parse('{"__proto__":{"mergeProbe":true}}'));
 assert.equal(Object.prototype.mergeProbe,undefined);assert.equal(Object.getPrototypeOf(target),Object.prototype);
 assert.ok(Object.hasOwn(target,'__proto__'));assert.equal(target.__proto__.mergeProbe,true);
});
check('holdout','constructor.prototype stays inert data',()=>{
 const target={};deepMerge(target,JSON.parse('{"constructor":{"prototype":{"mergeProbe":true}}}'));
 assert.equal(Object.prototype.mergeProbe,undefined);assert.equal(target.constructor.prototype.mergeProbe,true);
});
check('holdout','inherited nested destination is not mutated',()=>{
 const inherited={options:{existing:1}},target=Object.create(inherited);
 deepMerge(target,{options:{added:2}});assert.deepEqual(inherited.options,{existing:1});assert.ok(Object.hasOwn(target,'options'));
});
check('holdout','nested poison key does not escape its destination',()=>{
 const target={config:{}};deepMerge(target,JSON.parse('{"config":{"__proto__":{"mergeProbe":true}}}'));
 assert.equal(Object.prototype.mergeProbe,undefined);assert.equal(Object.getPrototypeOf(target.config),Object.prototype);
});
check('anchors','ordinary nested merge retains identity and values',()=>{const t={a:{one:1}};assert.equal(deepMerge(t,{a:{two:2}}),t);assert.deepEqual(t,{a:{one:1,two:2}});});
check('anchors','scalar array and null semantics retained',()=>{const a=[1,2],t={};deepMerge(t,{a,n:null,s:'text'});assert.equal(t.a,a);assert.deepEqual(t,{a,n:null,s:'text'});});
check('anchors','multiple sources merge in order',()=>{assert.deepEqual(deepMerge({}, {a:1},{b:2},{a:3}),{a:3,b:2});});
console.log(JSON.stringify(results));
if([...results.holdout,...results.anchors].some(x=>!x.passed))process.exitCode=1;
