const o = { d: null, c: 'kiln', id: true };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.d = 6; o.extra = 'x'; console.log(o.d, o.extra, Object.keys(o).length);
const c = { ...o, d: 'over' }; console.log(c.d, JSON.stringify(c));
class K21 {
  constructor(v) { this.v = v; this.d = v * 2; }
  get21() { return this.v + this.d; }
}
class L21 extends K21 { get21() { return super.get21() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K21(j) : new L21(j));
console.log(arr.map(x => x.get21()), arr[1], arr[0] instanceof K21);
