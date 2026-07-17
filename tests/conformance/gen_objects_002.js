const o = { d: true, c: [1, 2] };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.d = 0; o.extra = 'x'; console.log(o.d, o.extra, Object.keys(o).length);
const c = { ...o, d: 'over' }; console.log(c.d, JSON.stringify(c));
class K2 {
  constructor(v) { this.v = v; this.d = v * 2; }
  get2() { return this.v + this.d; }
}
class L2 extends K2 { get2() { return super.get2() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K2(j) : new L2(j));
console.log(arr.map(x => x.get2()), arr[1], arr[0] instanceof K2);
