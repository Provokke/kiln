const o = { c: 98, d: [1, 2], name: 63, a: 30, b: true };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.d = 4; o.extra = 'x'; console.log(o.d, o.extra, Object.keys(o).length);
const c = { ...o, d: 'over' }; console.log(c.d, JSON.stringify(c));
class K25 {
  constructor(v) { this.v = v; this.d = v * 2; }
  get25() { return this.v + this.d; }
}
class L25 extends K25 { get25() { return super.get25() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K25(j) : new L25(j));
console.log(arr.map(x => x.get25()), arr[1], arr[0] instanceof K25);
