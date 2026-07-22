const o = { b: 58, a: null };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.b = 3; o.extra = 'x'; console.log(o.b, o.extra, Object.keys(o).length);
const c = { ...o, b: 'over' }; console.log(c.b, JSON.stringify(c));
class K7 {
  constructor(v) { this.v = v; this.b = v * 2; }
  get7() { return this.v + this.b; }
}
class L7 extends K7 { get7() { return super.get7() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K7(j) : new L7(j));
console.log(arr.map(x => x.get7()), arr[1], arr[0] instanceof K7);
