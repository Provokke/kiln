const o = { kind: true, value: null, b: 'x', f: true, e: 78 };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.f = 0; o.extra = 'x'; console.log(o.f, o.extra, Object.keys(o).length);
const c = { ...o, f: 'over' }; console.log(c.f, JSON.stringify(c));
class K15 {
  constructor(v) { this.v = v; this.f = v * 2; }
  get15() { return this.v + this.f; }
}
class L15 extends K15 { get15() { return super.get15() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K15(j) : new L15(j));
console.log(arr.map(x => x.get15()), arr[1], arr[0] instanceof K15);
