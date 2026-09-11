import crypto from 'node:crypto';

export const sha256 = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
export const phases = ['resolve changed paths', 'load audit mode', 'scan repository',
  'apply score policy', 'apply mode and baseline', 'render artifacts', 'write JSON and Markdown'];
const phases7 = ['resolve changed paths', 'load audit mode', 'scan repository',
  'apply mode and baseline', 'render artifacts', 'write JSON and Markdown'];

function labelsFor(total) {
  if (total === 8) return phases;
  if (total === 7) return phases7;
  return null;
}

function scoreLabel(label) {
  return /^score \d+ raw \d+ findings \d+$/.test(label);
}

function timelineOf(events) {
  const totals = [...new Set(events.map(e => e.total).filter(total => total === 7 || total === 8))];
  if (totals.length > 1) throw new Error('invalid phase timeline');
  if (totals[0] === 7 || totals[0] === 8) return { labels: labelsFor(totals[0]), last: totals[0] };
  const score = events.find(e => scoreLabel(e.label));
  if (score?.position === 7) return { labels: phases7, last: 7 };
  if (score?.position === 8) return { labels: phases, last: 8 };
  if (events.some(e => e.position === 4 && e.label === 'apply mode and baseline')) return { labels: phases7, last: 7 };
  return { labels: phases, last: 8 };
}

export function parsePhase(text) {
  const plain = text.replace(/\x1b\[[0-9;]*[A-Za-z]/g, '').trim();
  // Only bar or percent progress markers count. Scorecard lines such as
  // "35/100" must not be treated as audit phases.
  const progress = plain.match(/(?:\]|\d+%)\s+(\d+)\/(\d+)\s+(.+)$/);
  if (!progress) return null;
  const position = Number(progress[1]), total = Number(progress[2]), label = progress[3].trim();
  const labels = labelsFor(total);
  if (!labels || !((position > 0 && position < total && label === labels[position - 1])
      || (position === total && scoreLabel(label)))) {
    throw new Error('unsupported observed audit phase');
  }
  return { position, label, total };
}

export function reportSummary(report) {
  const integer = (v, min, max = Number.MAX_SAFE_INTEGER) => Number.isSafeInteger(v) && v >= min && v <= max;
  const d = report?.decision;
  if (!integer(report?.score, 0, 100) || !integer(report?.raw_score, 0, 100)
      || !Array.isArray(report.findings) || !d || typeof d.passed !== 'boolean'
      || !['pass', 'fail', 'advisory'].includes(d.status)
      || !integer(d.minimum_score, 0, 100) || !integer(d.hard_findings, 0)
      || !integer(d.soft_findings, 0) || (d.ratchet && typeof d.ratchet.passed !== 'boolean')) {
    throw new Error('invalid audit report: score or decision contract');
  }
  const pass = d.passed && d.status === 'pass' && report.score >= d.minimum_score
    && d.hard_findings === 0 && d.ratchet?.passed !== false;
  if (d.status === 'fail' && d.passed) throw new Error('contradictory audit decision');
  return { score: report.score, raw: report.raw_score, findings: report.findings.length,
    floor: d.minimum_score, policyPassed: pass, status: d.status };
}

export function validateRecording(recording, { allowSynthetic = false } = {}) {
  if (recording?.schema !== 1 || !['recorded-process', 'synthetic-test'].includes(recording.kind)) throw new Error('unsupported recording');
  if (recording.kind === 'synthetic-test' && !allowSynthetic) throw new Error('synthetic input requires --allow-synthetic-test');
  if (!recording.identity || !/^[a-f0-9]{64}$/.test(recording.identity.executableSha256 || '')
      || !Array.isArray(recording.events) || !recording.events.length) throw new Error('missing recording identity or events');
  if (recording.events[0].atMs !== 0 || recording.events[0].position !== 0 || recording.events.length > 64) throw new Error('invalid initial phase or event bound');
  const { labels, last: terminal } = timelineOf(recording.events);
  let last = -1, position = 0;
  for (const e of recording.events) {
    if (!Number.isSafeInteger(e.atMs) || e.atMs < last || e.atMs < 0 || e.atMs > 600000
        || !Number.isSafeInteger(e.position) || e.position < position || e.position > terminal
        || (e.total != null && e.total !== terminal)
        || typeof e.label !== 'string' || e.label.length > 120) throw new Error('invalid phase timeline');
    if (e.position > 0 && e.position < terminal && e.label !== labels[e.position - 1]) throw new Error('unsupported phase identity');
    if (e.position === terminal && !scoreLabel(e.label)) throw new Error('invalid terminal phase');
    last = e.atMs; position = e.position;
  }
  const r = recording.result;
  if (!r || !Number.isSafeInteger(r.atMs) || r.atMs < last || r.atMs > 600000
      || !(Number.isInteger(r.exitCode) || r.exitCode === null)
      || !(typeof r.signal === 'string' || r.signal === null)) throw new Error('invalid execution result');
  if (r.report !== null) {
    if (!/^[a-f0-9]{64}$/.test(r.reportSha256 || '')) throw new Error('missing report digest');
    reportSummary(r.report);
  }
  if (r.exitCode === 0 && r.signal === null && !r.error && r.report === null) throw new Error('successful recording lacks report');
  return recording;
}

export function outcome(recording) {
  const r = recording.result;
  const summary = r.report ? reportSummary(r.report) : null;
  return { summary, passed: r.exitCode === 0 && r.signal === null && !r.error && summary?.policyPassed === true };
}
