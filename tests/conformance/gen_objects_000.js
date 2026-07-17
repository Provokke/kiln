const o = { e: null, b: [1, 2] };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.e = 4; o.extra = 'x'; console.log(o.e, o.extra, Object.keys(o).length);
const c = { ...o, e: 'over' }; console.log(c.e, JSON.stringify(c));
class K0 {
  constructor(v) { this.v = v; this.e = v * 2; }
  get0() { return this.v + this.e; }
}
class L0 extends K0 { get0() { return super.get0() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K0(j) : new L0(j));
console.log(arr.map(x => x.get0()), arr[1], arr[0] instanceof K0);
