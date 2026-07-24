const o = { kind: 28, f: 54 };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.f = 3; o.extra = 'x'; console.log(o.f, o.extra, Object.keys(o).length);
const c = { ...o, f: 'over' }; console.log(c.f, JSON.stringify(c));
class K11 {
  constructor(v) { this.v = v; this.f = v * 2; }
  get11() { return this.v + this.f; }
}
class L11 extends K11 { get11() { return super.get11() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K11(j) : new L11(j));
console.log(arr.map(x => x.get11()), arr[1], arr[0] instanceof K11);
