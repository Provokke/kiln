function make16(start) {
  let count = start;
  const log = [];
  return {
    inc: (d) => { count += d; log.push(count); return count; },
    get: () => count,
    history: () => log.slice(),
  };
}
const cs = [];
for (let j = 0; j < 3; j++) cs.push(make16(j * 3));
for (let r = 0; r < 3; r++) for (const c of cs) c.inc(r + 1);
console.log(cs.map(c => c.get()));
console.log(cs[0].history(), cs[2].history().length);
const adders = [];
for (let j = 0; j < 3; j++) { const base = j * j; adders.push(x => x + base); }
console.log(adders.map(f => f(3)));
