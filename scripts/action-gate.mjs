import fs from 'node:fs';
import { pathToFileURL } from 'node:url';

const object = value => value !== null && typeof value === 'object' && !Array.isArray(value);
const integer = (value, minimum, maximum) => Number.isSafeInteger(value) && value >= minimum && value <= maximum;
const strings = value => Array.isArray(value) && value.every(item => typeof item === 'string');
function requireValue(condition, message) {
  if (!condition) throw new Error(message);
}

export function parseFloor(value) {
  requireValue(typeof value === 'string' && /^(?:0|[1-9][0-9]?|100)$/.test(value), 'fail-under must be an integer from 0 to 100');
  return Number(value);
}

const MAX_REPORT_BYTES = 32 * 1024 * 1024;
function readReport(file) {
  const fd = fs.openSync(file, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW);
  try {
    const stat = fs.fstatSync(fd);
    requireValue(stat.isFile(), 'fresh report must be a regular file');
    requireValue(stat.size <= MAX_REPORT_BYTES, 'report exceeds the 32 MiB size limit');
    // Read at most the checked size plus one byte, even if a writer grows the file.
    const buffer = Buffer.allocUnsafe(stat.size + 1);
    let used = 0;
    while (used < buffer.length) {
      const count = fs.readSync(fd, buffer, used, buffer.length - used, null);
      if (count === 0) break;
      used += count;
    }
    requireValue(used === stat.size, 'report size changed during reading');
    return buffer.subarray(0, used).toString('utf8');
  } finally { fs.closeSync(fd); }
}

// JSON.parse accepts duplicate object keys; an enforcement record must not.
export function parseReport(text) {
  const result = JSON.parse(text);
  let at = 0;
  const whitespace = () => { while (/\s/.test(text[at] ?? '') && at < text.length) at++; };
  function string() {
    const start = at++;
    while (text[at] !== '"') { if (text[at++] === '\\') at++; }
    return JSON.parse(text.slice(start, ++at));
  }
  function value(depth = 0) {
    requireValue(depth < 256, 'report nesting exceeds supported limit');
    whitespace();
    if (text[at] === '{') {
      at++; whitespace(); const keys = new Set();
      while (text[at] !== '}') {
        const key = string();
        requireValue(!keys.has(key), `duplicate report field: ${key}`);
        keys.add(key); whitespace(); at++; value(depth + 1); whitespace();
        if (text[at] !== ',') break;
        at++; whitespace();
      }
      at++;
    } else if (text[at] === '[') {
      at++; whitespace();
      while (text[at] !== ']') {
        value(depth + 1); whitespace();
        if (text[at] !== ',') break;
        at++; whitespace();
      }
      at++;
    } else if (text[at] === '"') string();
    else while (at < text.length && !/[\s,}\]]/.test(text[at])) at++;
  }
  value();
  return result;
}

export function enforceReport(report, requestedFloor, mode) {
  requireValue(['standard', 'advisory', 'ratchet', 'release'].includes(mode), 'unsupported audit mode');
  const requested = parseFloor(requestedFloor);
  requireValue(object(report) && integer(report.score, 0, 100), 'report score must be an integer from 0 to 100');
  requireValue(integer(report.raw_score, 0, 100), 'invalid raw score');
  requireValue(object(report.scope) && report.scope.mode === 'full', 'gate requires a full audit');
  const decision = report.decision;
  requireValue(object(decision), 'missing audit decision');
  requireValue(integer(decision.minimum_score, 0, 100), 'invalid policy score floor');
  if (Object.hasOwn(report, 'policy')) {
    requireValue(object(report.policy) && integer(report.policy.minimum_score, 0, 100), 'invalid report policy score floor');
    requireValue(report.policy.minimum_score === decision.minimum_score, 'report policy and decision score floors disagree');
  }
  requireValue(typeof decision.passed === 'boolean', 'invalid decision passed flag');
  requireValue(integer(decision.hard_findings, 0, Number.MAX_SAFE_INTEGER), 'invalid hard finding count');
  requireValue(integer(decision.soft_findings, 0, Number.MAX_SAFE_INTEGER), 'invalid soft finding count');
  requireValue(['pass', 'fail', 'advisory'].includes(decision.status), 'invalid decision status');
  requireValue((mode === 'advisory') === (decision.status === 'advisory'), 'decision status contradicts requested mode');
  requireValue(decision.passed && decision.status !== 'fail' && decision.hard_findings === 0, 'audit policy did not pass');
  if (decision.ratchet != null) {
    const ratchet = decision.ratchet;
    requireValue(object(ratchet) && typeof ratchet.passed === 'boolean', 'invalid ratchet decision');
    requireValue(integer(ratchet.baseline_score, 0, 100) && integer(ratchet.allowed_drop, 0, Number.MAX_SAFE_INTEGER), 'invalid ratchet score limits');
    requireValue(integer(ratchet.score_delta, -100, 100) && ratchet.score_delta === report.score - ratchet.baseline_score, 'invalid ratchet score delta');
    requireValue(strings(ratchet.new_caps) && strings(ratchet.new_hard_findings) && typeof ratchet.policy_changed === 'boolean', 'invalid ratchet change fields');
    for (const name of ['baseline_report_fingerprint', 'baseline_input_fingerprint', 'baseline_policy_fingerprint']) {
      requireValue(typeof ratchet[name] === 'string' && /^sha256:[0-9a-f]{64}$/.test(ratchet[name]), `invalid ratchet ${name}`);
    }
    requireValue(ratchet.passed, 'ratchet did not pass');
  } else requireValue(!['ratchet', 'release'].includes(mode), 'required ratchet decision is missing');
  const floor = Math.max(requested, decision.minimum_score);
  requireValue(report.score >= floor, `score ${report.score} is below required floor ${floor}`);
  return { score: report.score, floor };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const args = process.argv.slice(2);
    if (args[0] === '--validate-floor' && args.length === 2) parseFloor(args[1]);
    else {
      requireValue(args.length === 3, 'usage: action-gate.mjs REPORT FLOOR MODE');
      const stat = fs.lstatSync(args[0]);
      requireValue(stat.isFile() && !stat.isSymbolicLink(), 'fresh report must be a regular file');
      const result = enforceReport(parseReport(readReport(args[0])), args[1], args[2]);
      console.log(`Jankurai score ${result.score}; required floor ${result.floor}: passed`);
    }
  } catch (error) {
    console.error(`Jankurai gate: ${error.message}`);
    process.exitCode = 1;
  }
}
