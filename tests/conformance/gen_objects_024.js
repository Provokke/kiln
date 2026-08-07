const o = { b: null, e: [1, 2], kind: null };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.b = 2; o.extra = 'x'; console.log(o.b, o.extra, Object.keys(o).length);
const c = { ...o, b: 'over' }; console.log(c.b, JSON.stringify(c));
class K24 {
  constructor(v) { this.v = v; this.b = v * 2; }
  get24() { return this.v + this.b; }
}
class L24 extends K24 { get24() { return super.get24() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K24(j) : new L24(j));
console.log(arr.map(x => x.get24()), arr[1], arr[0] instanceof K24);
