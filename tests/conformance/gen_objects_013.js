const o = { d: 'beta', f: 'alpha', c: null, value: null, e: [1, 2] };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.e = 6; o.extra = 'x'; console.log(o.e, o.extra, Object.keys(o).length);
const c = { ...o, e: 'over' }; console.log(c.e, JSON.stringify(c));
class K13 {
  constructor(v) { this.v = v; this.e = v * 2; }
  get13() { return this.v + this.e; }
}
class L13 extends K13 { get13() { return super.get13() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K13(j) : new L13(j));
console.log(arr.map(x => x.get13()), arr[1], arr[0] instanceof K13);
