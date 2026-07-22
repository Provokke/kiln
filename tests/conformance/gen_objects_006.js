const o = { b: null, f: 55, a: 10, kind: 47, name: null };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.a = 2; o.extra = 'x'; console.log(o.a, o.extra, Object.keys(o).length);
const c = { ...o, a: 'over' }; console.log(c.a, JSON.stringify(c));
class K6 {
  constructor(v) { this.v = v; this.a = v * 2; }
  get6() { return this.v + this.a; }
}
class L6 extends K6 { get6() { return super.get6() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K6(j) : new L6(j));
console.log(arr.map(x => x.get6()), arr[1], arr[0] instanceof K6);
