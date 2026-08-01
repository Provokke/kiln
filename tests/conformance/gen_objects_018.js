const o = { b: true, e: null, value: 35, d: 'delta' };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.e = 3; o.extra = 'x'; console.log(o.e, o.extra, Object.keys(o).length);
const c = { ...o, e: 'over' }; console.log(c.e, JSON.stringify(c));
class K18 {
  constructor(v) { this.v = v; this.e = v * 2; }
  get18() { return this.v + this.e; }
}
class L18 extends K18 { get18() { return super.get18() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K18(j) : new L18(j));
console.log(arr.map(x => x.get18()), arr[1], arr[0] instanceof K18);
