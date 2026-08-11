const o = { f: null, id: 27, b: 'beta', d: null, name: 'kiln' };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.d = 7; o.extra = 'x'; console.log(o.d, o.extra, Object.keys(o).length);
const c = { ...o, d: 'over' }; console.log(c.d, JSON.stringify(c));
class K29 {
  constructor(v) { this.v = v; this.d = v * 2; }
  get29() { return this.v + this.d; }
}
class L29 extends K29 { get29() { return super.get29() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K29(j) : new L29(j));
console.log(arr.map(x => x.get29()), arr[1], arr[0] instanceof K29);
