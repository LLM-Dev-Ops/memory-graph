import fs from 'node:fs';
import {spawnSync,execFileSync} from 'node:child_process';
import {resolve,join} from 'node:path';
import {createHash} from 'node:crypto';
const [base,head,tools,out]=process.argv.slice(2).map(x=>resolve(x));
const hash=x=>createHash('sha256').update(x).digest('hex');
fs.mkdirSync(out+'.logs',{recursive:true});
function run(root,label,key,cmd,args,seconds=40){
 const start=Date.now(),r=spawnSync('timeout',['--kill-after=5',String(seconds),cmd,...args],{cwd:root,encoding:'utf8',maxBuffer:16*1024*1024,timeout:(seconds+8)*1000});
 const text=(r.stdout||'')+(r.stderr||'');fs.writeFileSync(join(out+'.logs',label+'-'+key+'.log'),text);
 return {command:['timeout','--kill-after=5',String(seconds),cmd,...args],exit:r.status,signal:r.signal,error:r.error?.message,ms:Date.now()-start,pass:r.status===0&&!r.error,sha256:hash(text),stdout:r.stdout||''};
}
function measure(root,label){
 const checks={probe:run(root,label,'probe',process.execPath,[join(head,'scripts/probe-sdk-merge.mjs'),root],10),rootTests:run(root,label,'root','npm',['test'],40)};
 const probe=JSON.parse(checks.probe.stdout);
 const sdk=join(root,'clients/typescript');
 checks.sdkBuild=run(sdk,label,'sdk-build','npm',['run','build'],30);
 checks.sdkUtils=run(sdk,label,'sdk-utils',process.execPath,['node_modules/jest/bin/jest.js','--runInBand','--runTestsByPath','tests/unit/utils.test.ts','--json','--outputFile',out+'.'+label+'-utils.json'],35);
 checks.sdkUnit=run(sdk,label,'sdk-unit',process.execPath,['node_modules/jest/bin/jest.js','--runInBand','--testPathPatterns=tests/unit','--json','--outputFile',out+'.'+label+'-unit.json'],45);
 checks.audit=run(root,label,'npm-audit','npm',['audit','--json'],20);
 checks.sdkAudit=run(sdk,label,'sdk-audit','npm',['audit','--json'],20);
 checks.rust=run(root,label,'rust','cargo',['test','--workspace','--all-features','--locked'],10);
 checks.ruflo=run(root,label,'ruflo','ruflo',['--version'],10);
 return {probe,checks};
}
const baseline=measure(base,'baseline'),candidate=measure(head,'candidate');
const lib=join(tools,'node_modules/@metaharness/flywheel');
const fw=await import(join(lib,'dist/index.js'));
const rev=root=>execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim();
const b=rev(base),h=rev(head),path='clients/typescript/src/utils.ts',parentBytes=fs.readFileSync(join(base,path),'utf8');
function score(m,s){const a=m.probe[s==='holdout'?'holdout':'anchors'],primary=a.filter(x=>x.passed).length/a.length;return {primary,noopRate:1-primary,costPerWin:0,regressed:Object.values(m.checks).some(x=>!x.pass)};}
const result=await fw.runFlywheelGenerations({rootPolicy:{revision:b},proposer:async()=>({value:h,summary:'Prevent SDK deepMerge prototype pollution using own destinations and data properties',inverse:{path,parentBytes,sha256:hash(parentBytes)}}),evaluator:async(p,s)=>score(p.revision===b?baseline:candidate,s.id),holdout:{id:'holdout',items:candidate.probe.holdout.map(x=>x.name)},anchor:{id:'anchor',items:candidate.probe.anchors.map(x=>x.name)},mutationTargets:['revision'],maxGenerations:1,signer:fw.makeSigner(),promotionRule:fw.meetsPromotionRule,dataSource:'LIVE'});
const fingerprint=fw.gateFingerprint(fw.meetsPromotionRule),replay=fw.verifyReplayBundle(result.replayBundle,{pinnedGateFingerprint:fingerprint});
for(const m of [baseline,candidate])for(const check of Object.values(m.checks))delete check.stdout;
fs.writeFileSync(out,JSON.stringify({baselineRevision:b,candidateRevision:h,baseline,candidate,result,replay,fingerprint,rollback:{path,parentBytes,sha256:hash(parentBytes)},limits:'Node24 transforms actual TS utility module with identical erased-import normalization. Real tsc/Jest checks separate. No deployed gRPC integration or Rust proof, missing commands fail closed. Neutral unmeasured cost; noopRate=failed-case fraction. Shared dependency trees only after lock bytes compared.'},null,2));
console.log(JSON.stringify({promotions:result.promotions.length,replay,baseline:score(baseline,'holdout'),candidate:score(candidate,'holdout')}));
